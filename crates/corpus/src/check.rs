//! Cross-file checks over a loaded corpus.

use crate::drift::{Disagreement, find_drift};
use crate::entry::entry_path;
use crate::load::Corpus;
use cast_iron_schema::headings::keyed_headings;
use cast_iron_schema::{
    Block, BookNode, EntryId, EntryKind, Relation, RelationKind, RuleEntry, TypedFields,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The creature types of Pathfinder First Edition, in the corpus spelling.
///
/// The client shows a `creature_type` facet value as written, so one type must have exactly one
/// spelling. Case variants such as `Humanoid` would otherwise appear as separate filter values.
pub const CREATURE_TYPES: [&str; 13] = [
    "aberration",
    "animal",
    "construct",
    "dragon",
    "fey",
    "humanoid",
    "magicalBeast",
    "monstrousHumanoid",
    "ooze",
    "outsider",
    "plant",
    "undead",
    "vermin",
];

/// What a check found. Every problem fails the run; there are no warnings.
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub problems: Vec<String>,
    /// Values that `[fields]` and the body both state and that disagree. Each one is also
    /// listed in `problems`.
    pub drift: Vec<Disagreement>,
}

impl Report {
    /// Disagreements per entry kind, by kind slug.
    pub fn drift_by_kind(&self) -> BTreeMap<&str, usize> {
        let mut counts = BTreeMap::new();
        for item in &self.drift {
            *counts.entry(item.kind.as_str()).or_default() += 1;
        }
        counts
    }

    /// The drift list as a report file: counts per kind, then one line per disagreement.
    pub fn drift_report(&self) -> String {
        let mut out = String::from("# Corpus drift report\n\n");
        out.push_str(
            "Values that `[fields]` and the body both state and that disagree. Each line names both places.\n\n",
        );
        out.push_str("| Kind | Disagreements |\n|---|---|\n");
        for (kind, count) in self.drift_by_kind() {
            out.push_str(&format!("| {kind} | {count} |\n"));
        }
        out.push_str(&format!("| **total** | **{}** |\n\n", self.drift.len()));
        let mut sorted: Vec<&Disagreement> = self.drift.iter().collect();
        sorted.sort_by(|a, b| (&a.kind, &a.id, a.field).cmp(&(&b.kind, &b.id, b.field)));
        for item in sorted {
            out.push_str(&item.describe());
            out.push('\n');
        }
        out
    }
}

/// The 1-based line of the first line of `text` that holds `needle`, or 1.
pub fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map_or(1, |index| index + 1)
}

/// Every 1-based line of `text` that holds `needle`, in file order.
fn lines_of(text: &str, needle: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains(needle))
        .map(|(index, _)| index + 1)
        .collect()
}

/// Every heading of an entry with its URL anchor key and plain text.
pub fn heading_keys(entry: &RuleEntry) -> Vec<(String, String)> {
    keyed_headings(&entry.body)
        .into_iter()
        .filter_map(|(block, key)| {
            let key = key?;
            let Block::Heading { content, .. } = block else {
                return None;
            };
            Some((key, cast_iron_schema::headings::plain_text(content)))
        })
        .collect()
}

/// The body part of an entry file, with the 1-based line number it starts on.
fn body_text(text: &str) -> (usize, &str) {
    let mut offset = 0;
    let mut fences = 0;
    for line in text.split_inclusive('\n') {
        offset += line.len();
        if line.trim_end() == "+++" {
            fences += 1;
            if fences == 2 {
                break;
            }
        }
    }
    let start = text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
    (start, &text[offset..])
}

/// Upstream inline tokens the retired import left in body text: `&`, one or more ASCII
/// uppercase letters, and `&`, so `&L&` and `&FN&` are caught. Lowercase HTML-style entities
/// (`&times&`, `&quot&`, ...) and mixed-case markers (`&Next1&`) are deliberately not matched.
fn upstream_markup_tokens(text: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b'&' => {
                let start = i;
                let mut end = i + 1;
                while end < bytes.len() && bytes[end].is_ascii_uppercase() {
                    end += 1;
                }
                if end > i + 1 && end < bytes.len() && bytes[end] == b'&' {
                    out.push((line, text[start..=end].to_string()));
                    i = end + 1;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// Every string that `[fields]` shows to a reader, paired with the key that holds it.
///
/// Handles and numbers are not display text: `prerequisite_feats`, `levels[].level`, `cr`,
/// and `alternate` stay out. Every remaining string is scanned, including pair labels and
/// values and the `levels[].class` names.
fn field_display_texts(fields: &TypedFields) -> Vec<(&'static str, &str)> {
    fn optional<'a>(
        out: &mut Vec<(&'static str, &'a str)>,
        key: &'static str,
        value: &'a Option<String>,
    ) {
        if let Some(value) = value {
            out.push((key, value.as_str()));
        }
    }
    fn list<'a>(out: &mut Vec<(&'static str, &'a str)>, key: &'static str, values: &'a [String]) {
        out.extend(values.iter().map(|value| (key, value.as_str())));
    }
    fn pairs<'a>(
        out: &mut Vec<(&'static str, &'a str)>,
        key: &'static str,
        values: &'a [(String, String)],
    ) {
        for (label, value) in values {
            out.push((key, label.as_str()));
            out.push((key, value.as_str()));
        }
    }

    let mut out = Vec::new();
    match fields {
        TypedFields::Spell(fields) => {
            out.push(("school", fields.school.as_str()));
            list(&mut out, "subschools", &fields.subschools);
            list(&mut out, "descriptors", &fields.descriptors);
            out.extend(
                fields
                    .levels
                    .iter()
                    .map(|level| ("levels", level.class.as_str())),
            );
            optional(&mut out, "casting_time", &fields.casting_time);
            optional(&mut out, "components", &fields.components);
            optional(&mut out, "range", &fields.range);
            pairs(&mut out, "effects", &fields.effects);
            optional(&mut out, "duration", &fields.duration);
            optional(&mut out, "saving_throw", &fields.saving_throw);
            optional(&mut out, "spell_resistance", &fields.spell_resistance);
        }
        TypedFields::Feat(fields) => {
            list(&mut out, "feat_types", &fields.feat_types);
            optional(&mut out, "prerequisites", &fields.prerequisites);
            optional(&mut out, "benefit", &fields.benefit);
            optional(&mut out, "normal", &fields.normal);
            optional(&mut out, "special", &fields.special);
        }
        TypedFields::Creature(fields) => {
            optional(&mut out, "creature_type", &fields.creature_type);
            list(&mut out, "subtypes", &fields.subtypes);
            optional(&mut out, "size", &fields.size);
            optional(&mut out, "alignment", &fields.alignment);
            optional(&mut out, "ac", &fields.ac);
            optional(&mut out, "hp", &fields.hp);
            optional(&mut out, "fortitude", &fields.fortitude);
            optional(&mut out, "reflex", &fields.reflex);
            optional(&mut out, "will", &fields.will);
            optional(&mut out, "melee", &fields.melee);
            optional(&mut out, "ranged", &fields.ranged);
            optional(&mut out, "special_attacks", &fields.special_attacks);
            optional(&mut out, "special_qualities", &fields.special_qualities);
            if let Some(abilities) = &fields.abilities {
                out.extend(abilities.iter().map(|value| ("abilities", value.as_str())));
            }
        }
        TypedFields::Item(fields) => {
            optional(&mut out, "category", &fields.category);
            optional(&mut out, "price", &fields.price);
            optional(&mut out, "weight", &fields.weight);
            optional(&mut out, "slot", &fields.slot);
            optional(&mut out, "aura", &fields.aura);
            optional(&mut out, "caster_level", &fields.caster_level);
            pairs(&mut out, "attributes", &fields.attributes);
        }
        TypedFields::Class(fields) => {
            optional(&mut out, "class_type", &fields.class_type);
            optional(&mut out, "hit_die", &fields.hit_die);
            optional(&mut out, "skill_ranks", &fields.skill_ranks);
            optional(&mut out, "class_skills", &fields.class_skills);
        }
        TypedFields::Trait(fields) => {
            out.push(("category", fields.category.as_str()));
            optional(&mut out, "detail", &fields.detail);
        }
        TypedFields::Archetype(fields) => {
            list(&mut out, "replaces", &fields.replaces);
            list(&mut out, "alters", &fields.alters);
        }
        TypedFields::ClassOption(fields) => {
            list(&mut out, "replaces", &fields.replaces);
            list(&mut out, "alters", &fields.alters);
        }
        TypedFields::RacialTrait(_) | TypedFields::None => {}
    }
    out
}

/// A display string in an entry and the place in the file that holds it.
enum DisplayText<'a> {
    Title(&'a str),
    Tag(&'a str),
    Field(&'static str, &'a str),
}

impl<'a> DisplayText<'a> {
    fn value(&self) -> &'a str {
        match self {
            DisplayText::Title(value) | DisplayText::Tag(value) => value,
            DisplayText::Field(_, value) => value,
        }
    }

    /// What the finding calls the place, so a title or tag does not read as `[fields]`.
    fn place(&self) -> String {
        match self {
            DisplayText::Title(_) => "the entry title".into(),
            DisplayText::Tag(_) => "the entry tags".into(),
            DisplayText::Field(key, _) => format!("`[fields]` key `{key}`"),
        }
    }
}

/// Every string in an entry that a reader sees, with the place that holds it. Title, tags,
/// and typed fields share the rule; handles and numbers stay out.
fn entry_display_texts(entry: &RuleEntry) -> Vec<DisplayText<'_>> {
    let mut out = vec![DisplayText::Title(&entry.title)];
    out.extend(entry.tags.iter().map(|tag| DisplayText::Tag(tag)));
    out.extend(
        field_display_texts(&entry.fields)
            .into_iter()
            .map(|(key, value)| DisplayText::Field(key, value)),
    );
    out
}

pub fn fields_kind(fields: &TypedFields) -> &'static str {
    match fields {
        TypedFields::Spell(_) => "spell",
        TypedFields::Feat(_) => "feat",
        TypedFields::Creature(_) => "creature",
        TypedFields::Item(_) => "item",
        TypedFields::Class(_) => "class",
        TypedFields::Trait(_) => "trait",
        TypedFields::RacialTrait(_) => "racial_trait",
        TypedFields::Archetype(_) => "archetype",
        TypedFields::ClassOption(_) => "class_option",
        TypedFields::None => "none",
    }
}

pub fn fields_match(kind: EntryKind, fields: &TypedFields) -> bool {
    match fields {
        TypedFields::Spell(_) => kind == EntryKind::Spell,
        TypedFields::Feat(_) => kind == EntryKind::Feat,
        TypedFields::Creature(_) => kind == EntryKind::Creature,
        TypedFields::Item(_) => kind == EntryKind::Item,
        TypedFields::Class(_) => matches!(kind, EntryKind::Class | EntryKind::PrestigeClass),
        TypedFields::Trait(_) => kind == EntryKind::Trait,
        TypedFields::RacialTrait(_) => kind == EntryKind::RacialTrait,
        TypedFields::Archetype(_) => kind == EntryKind::Archetype,
        TypedFields::ClassOption(_) => kind == EntryKind::ClassOption,
        TypedFields::None => matches!(
            kind,
            EntryKind::Rule
                | EntryKind::Race
                | EntryKind::Skill
                | EntryKind::Affliction
                | EntryKind::Trap
                | EntryKind::Archetype
        ),
    }
}

/// The licensing gate: every publication states a copyright and is covered by a Section 15
/// notice or a reviewed `notice_pending` entry.
///
/// A publication is covered when the text of some `section_15` notice in `legal.toml` is part of
/// its `copyright`, which is how the import joined them, or when its title is in
/// `notice_pending`. A problem names `publications.toml`, the line, and the publication id.
fn check_publication_notices(corpus: &Corpus, report: &mut Report) {
    let text = std::fs::read_to_string(corpus.dir.join("publications.toml")).unwrap_or_default();
    for publication in &corpus.publications {
        let at = format!(
            "publications.toml:{}",
            line_of(&text, &format!("id = \"{}\"", publication.id))
        );
        let copyright = publication.copyright.as_deref().map_or("", str::trim);
        if copyright.is_empty() {
            report.problems.push(format!(
                "{at}: publication `{}` has no copyright",
                publication.id
            ));
            continue;
        }
        let pending = corpus.legal.notice_pending.contains(&publication.title);
        let noticed = corpus.legal.section_15.iter().any(|notice| {
            let notice = notice.notice.trim();
            !notice.is_empty() && copyright.contains(notice)
        });
        if !pending && !noticed {
            report.problems.push(format!(
                "{at}: publication `{}` ({}) is covered by no section_15 notice in legal.toml and is not in notice_pending",
                publication.id, publication.title
            ));
        }
    }
}

/// Runs every cross-file check.
pub fn check(corpus: &Corpus) -> Report {
    let mut report = Report::default();
    let publication_ids: BTreeSet<&str> = corpus
        .publications
        .iter()
        .map(|publication| publication.id.as_str())
        .collect();
    let mut license_ids: BTreeSet<&str> = corpus
        .legal
        .licenses
        .iter()
        .map(|license| license.id.as_str())
        .collect();
    if !corpus.legal.community_use_notice.is_empty() {
        license_ids.insert("legal#community-use");
    }
    let ids: BTreeSet<&str> = corpus.entries.keys().map(String::as_str).collect();
    let keys: BTreeMap<&str, Vec<(String, String)>> = corpus
        .entries
        .iter()
        .map(|(id, stored)| (id.as_str(), heading_keys(&stored.entry)))
        .collect();

    check_publication_notices(corpus, &mut report);

    for (id, stored) in &corpus.entries {
        let entry = &stored.entry;
        let path = entry_path(&entry.id);
        if stored.path != path {
            report.problems.push(format!(
                "{}: entry id `{}` belongs at `{}`, not `{}`",
                stored.path.display(),
                entry.id,
                path.display(),
                stored.path.display()
            ));
        }
        let at = |needle: &str| format!("{}:{}", path.display(), line_of(&stored.text, needle));
        if !fields_match(entry.kind, &entry.fields) {
            report.problems.push(format!(
                "{}: entry kind `{}` does not match `[fields]` kind `{}`",
                at("kind"),
                entry.kind.slug(),
                fields_kind(&entry.fields)
            ));
        }
        let (body_start, body) = body_text(&stored.text);
        for (line, token) in upstream_markup_tokens(body) {
            report.problems.push(format!(
                "{}:{}: left-over upstream token `{token}`. Convert it to plain text or a typed link.",
                path.display(),
                body_start + line - 1
            ));
        }
        // The writer puts each display string on one line. When the same value appears more
        // than once, consume the next matching line so every occurrence reports its own
        // place instead of the first one over and over.
        let mut matched: BTreeMap<String, usize> = BTreeMap::new();
        for text in entry_display_texts(entry) {
            let value = text.value();
            let tokens = upstream_markup_tokens(value);
            if tokens.is_empty() {
                continue;
            }
            let needle = crate::tomlutil::string(value);
            let lines = lines_of(&stored.text, &needle);
            let next = matched.entry(needle.clone()).or_default();
            let line = lines
                .get(*next)
                .copied()
                .unwrap_or_else(|| line_of(&stored.text, &needle));
            *next += 1;
            let place = text.place();
            for (_, token) in tokens {
                report.problems.push(format!(
                    "{}:{line}: left-over upstream token `{token}` in {place}. Convert it to plain text or a typed link.",
                    path.display()
                ));
            }
        }
        if let TypedFields::Creature(fields) = &entry.fields
            && let Some(creature_type) = &fields.creature_type
            && !CREATURE_TYPES.contains(&creature_type.as_str())
        {
            report.problems.push(format!(
                "{}: creature_type `{creature_type}` is not one of the PF1e creature types: {}",
                at(&format!("creature_type = \"{creature_type}\"")),
                CREATURE_TYPES.join(", ")
            ));
        }
        for source in &entry.source_ids {
            if !publication_ids.contains(source.as_str()) {
                report
                    .problems
                    .push(format!("{}: unknown publication `{source}`", at(source)));
            }
        }
        for license in &entry.license_refs {
            if !license_ids.contains(license.as_str()) {
                report
                    .problems
                    .push(format!("{}: unknown licence `{license}`", at(license)));
            }
        }
        if let Some(parent) = &entry.parent
            && !ids.contains(parent.as_str())
        {
            report.problems.push(format!(
                "{}: unknown parent `{parent}`",
                at(parent.as_str())
            ));
        }
        for link in &entry.links {
            let target = link.target.as_str();
            if !ids.contains(target) {
                report
                    .problems
                    .push(format!("{}: unknown link target `{target}`", at(target)));
                continue;
            }
            let Some(anchor) = &link.anchor else { continue };
            let available = &keys[target];
            if available.iter().any(|(key, _)| key == anchor) {
                continue;
            }
            let needle = format!("]({target}#{anchor})");
            report.problems.push(format!(
                "{}: the link to `{target}#{anchor}` points at a heading that does not exist. {}",
                at(&needle),
                anchor_fix(&corpus.entries[target].path, anchor, available)
            ));
        }
        if let TypedFields::Feat(fields) = &entry.fields {
            for feat in &fields.prerequisite_feats {
                if !ids.contains(feat.as_str()) {
                    report.problems.push(format!(
                        "{}: unknown prerequisite feat `{feat}`",
                        at(feat.as_str())
                    ));
                }
            }
        }
        for relation in &entry.relations {
            let target = relation.target.as_str();
            if !ids.contains(target) {
                report.problems.push(format!(
                    "{}: unknown relation target `{target}`",
                    at(target)
                ));
            }
        }
        let _ = id;
        for item in find_drift(stored) {
            report.problems.push(item.describe());
            report.drift.push(item);
        }
    }

    check_relations(corpus, &mut report);
    check_option_groups(corpus, &ids, &mut report);
    check_publication_suffixes(corpus, &ids, &mut report);

    for tree in &corpus.books.books {
        let mut placed = BTreeSet::new();
        for chapter in &tree.chapters {
            check_book_node(corpus, chapter, &ids, &keys, &mut placed, &mut report);
        }
    }
    if let Some(aliases) = &corpus.aliases {
        for entry in &aliases.entries {
            if !ids.contains(entry.target.as_str()) {
                report.problems.push(format!(
                    "aliases.toml:1: alias target `{}` is not an entry",
                    entry.target
                ));
            }
        }
    }
    report
}

/// The fix to suggest for a link anchor that no heading in the target has.
fn anchor_fix(target_path: &Path, anchor: &str, available: &[(String, String)]) -> String {
    if available.is_empty() {
        return format!(
            "`{}` has no headings. Remove `#{anchor}` from the link, or add a heading `## Title {{#{anchor}}}` to that file.",
            target_path.display()
        );
    }
    let shown: Vec<String> = available
        .iter()
        .take(8)
        .map(|(key, text)| format!("`{key}` (\"{text}\")"))
        .collect();
    let more = available.len().saturating_sub(8);
    format!(
        "`{}` has these headings: {}{}. Change the link to one of them, or add `{{#{anchor}}}` to the heading it should reach.",
        target_path.display(),
        shown.join(", "),
        if more > 0 {
            format!(" and {more} more")
        } else {
            String::new()
        }
    )
}

fn check_relations(corpus: &Corpus, report: &mut Report) {
    let mut kinds: BTreeMap<&str, EntryKind> = BTreeMap::new();
    let mut variant_edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut revision_keys: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let mut revision_ids: BTreeSet<&str> = BTreeSet::new();

    for stored in corpus.entries.values() {
        kinds.insert(stored.entry.id.as_str(), stored.entry.kind);
        if stored
            .entry
            .relations
            .iter()
            .any(|relation| relation.kind == RelationKind::Revision)
        {
            revision_ids.insert(stored.entry.id.as_str());
        }
    }

    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let at = |target: &str| {
            format!(
                "{}:{}",
                stored.path.display(),
                line_of(&stored.text, target)
            )
        };
        let mut variants = 0usize;
        for relation in &entry.relations {
            let line = at(relation.target.as_str());
            if let Some(problem) = relation_label_problem(relation) {
                report.problems.push(format!("{line}: {problem}"));
            }
            let Some(target_kind) = kinds.get(relation.target.as_str()).copied() else {
                continue;
            };
            match relation.kind {
                RelationKind::Variant => {
                    variants += 1;
                    // TODO(slice 5, #143): error when a spell body holds another spell's title
                    // as a heading.
                    if entry.kind != EntryKind::Spell || target_kind != EntryKind::Spell {
                        report.problems.push(format!(
                            "{line}: a `variant` relation must go from a spell to a spell, but `{}` is a {} and `{}` is a {}",
                            entry.id,
                            entry.kind.slug(),
                            relation.target,
                            target_kind.slug()
                        ));
                    }
                    variant_edges
                        .entry(entry.id.to_string())
                        .or_default()
                        .push(relation.target.to_string());
                }
                RelationKind::Revision => {
                    let target = &corpus.entries[relation.target.as_str()];
                    if target
                        .entry
                        .relations
                        .iter()
                        .any(|row| row.kind == RelationKind::Revision)
                    {
                        report.problems.push(format!(
                            "{line}: a `revision` cannot point at `{}`, which is itself a revision",
                            relation.target
                        ));
                    }
                    if target_kind != entry.kind {
                        report.problems.push(format!(
                            "{line}: a `revision` must keep the target's kind, but `{}` is a {} and `{}` is a {}",
                            entry.id,
                            entry.kind.slug(),
                            relation.target,
                            target_kind.slug()
                        ));
                    }
                    if entry
                        .source_ids
                        .iter()
                        .any(|source| target.entry.source_ids.contains(source))
                    {
                        report.problems.push(format!(
                            "{line}: a `revision` of `{}` shares a book with it; a revision must come from another publication",
                            relation.target
                        ));
                    }
                    let label = relation.label.as_deref();
                    if matches!(label, Some("removed" | "merged"))
                        && !fields_are_empty(&entry.fields)
                    {
                        report.problems.push(format!(
                            "{line}: a `{}` revision must have empty `[fields]`",
                            label.unwrap_or_default()
                        ));
                    }
                    if label == Some("merged")
                        && !entry
                            .links
                            .iter()
                            .any(|link| kinds.get(link.target.as_str()) == Some(&EntryKind::Feat))
                    {
                        report.problems.push(format!(
                            "{line}: a `merged` revision must link the feat it merges into"
                        ));
                    }
                    if entry.source_ids.len() == 1 {
                        revision_keys
                            .entry((relation.target.to_string(), entry.source_ids[0].to_string()))
                            .or_default()
                            .push(entry.id.to_string());
                    }
                }
                RelationKind::Addon => {
                    if relation.label.as_deref() == Some("mythic") {
                        // The rule is about feats only. A spell or rule holder (the mythic-spell
                        // add-ons of slice 3) is not required to carry Mythic in `feat_types`.
                        if entry.kind == EntryKind::Feat {
                            let mythic = matches!(
                                &entry.fields,
                                TypedFields::Feat(fields)
                                    if fields.feat_types.iter().any(|kind| kind == "Mythic")
                            );
                            if !mythic {
                                report.problems.push(format!(
                                    "{line}: a `mythic` add-on must be a feat whose `feat_types` include `Mythic`"
                                ));
                            }
                        }
                        // TODO(slice 2, #178): error when a base still holds a `Mythic <Title>`
                        // heading while an add-on targets it.
                    }
                }
                RelationKind::Part | RelationKind::Archetype | RelationKind::Prerequisite => {}
            }
        }
        if variants > 1 {
            report.problems.push(format!(
                "{}: entry `{}` holds {variants} `variant` relations; at most one is allowed",
                stored.path.display(),
                entry.id
            ));
        }
        for link in &entry.links {
            if revision_ids.contains(link.target.as_str()) {
                report.problems.push(format!(
                    "{}: the body links `{}`, which is a revision",
                    stored.path.display(),
                    link.target
                ));
            }
        }
        if let TypedFields::Feat(fields) = &entry.fields {
            for feat in &fields.prerequisite_feats {
                if revision_ids.contains(feat.as_str()) {
                    report.problems.push(format!(
                        "{}: `prerequisite_feats` names `{feat}`, which is a revision",
                        stored.path.display()
                    ));
                }
            }
        }
    }

    for (target, entries) in &revision_keys {
        for extra in entries.iter().skip(1) {
            let path = EntryId::parse(extra.clone())
                .map(|id| entry_path(&id))
                .unwrap_or_else(|_| std::path::PathBuf::from(extra));
            report.problems.push(format!(
                "{}: `{}` is a second revision of `{}` in publication `{}`",
                path.display(),
                extra,
                target.0,
                target.1
            ));
        }
    }

    check_variant_cycles(&variant_edges, report);
}

/// The label vocabulary is closed: a relation may carry only the labels its kind declares.
fn relation_label_problem(relation: &Relation) -> Option<String> {
    let label = relation.label.as_deref();
    let allowed = match relation.kind {
        RelationKind::Prerequisite => matches!(label, None | Some("or")),
        RelationKind::Part => matches!(label, None | Some("alternate")),
        RelationKind::Addon => matches!(label, Some("mythic" | "stamina" | "changes")),
        RelationKind::Revision => matches!(label, None | Some("removed" | "merged")),
        RelationKind::Variant | RelationKind::Archetype => label.is_none(),
    };
    if allowed {
        return None;
    }
    if relation.kind == RelationKind::Addon {
        return Some(
            "an `addon` relation needs exactly one of `mythic`, `stamina` or `changes`".into(),
        );
    }
    Some(match label {
        Some(label) => format!(
            "`{label}` is not a valid label on a `{}` relation",
            relation.kind.slug()
        ),
        None => format!("a `{}` relation needs a label", relation.kind.slug()),
    })
}

fn fields_are_empty(fields: &TypedFields) -> bool {
    match fields {
        TypedFields::Spell(fields) => {
            fields.school.is_empty()
                && fields.subschools.is_empty()
                && fields.descriptors.is_empty()
                && fields.levels.is_empty()
                && fields.casting_time.is_none()
                && fields.components.is_none()
                && fields.range.is_none()
                && fields.effects.is_empty()
                && fields.duration.is_none()
                && fields.saving_throw.is_none()
                && fields.spell_resistance.is_none()
        }
        TypedFields::Feat(fields) => {
            fields.feat_types.is_empty()
                && fields.prerequisites.is_none()
                && fields.prerequisite_feats.is_empty()
                && fields.benefit.is_none()
                && fields.normal.is_none()
                && fields.special.is_none()
        }
        TypedFields::Creature(fields) => {
            fields.cr.is_none()
                && fields.creature_type.is_none()
                && fields.subtypes.is_empty()
                && fields.size.is_none()
                && fields.alignment.is_none()
                && fields.ac.is_none()
                && fields.hp.is_none()
                && fields.fortitude.is_none()
                && fields.reflex.is_none()
                && fields.will.is_none()
                && fields.melee.is_none()
                && fields.ranged.is_none()
                && fields.special_attacks.is_none()
                && fields.special_qualities.is_none()
                && fields.abilities.is_none()
        }
        TypedFields::Item(fields) => {
            fields.category.is_none()
                && fields.price.is_none()
                && fields.weight.is_none()
                && fields.slot.is_none()
                && fields.aura.is_none()
                && fields.caster_level.is_none()
                && fields.attributes.is_empty()
        }
        TypedFields::Class(fields) => {
            fields.class_type.is_none()
                && fields.hit_die.is_none()
                && fields.skill_ranks.is_none()
                && fields.class_skills.is_none()
        }
        TypedFields::Trait(fields) => fields.category.is_empty() && fields.detail.is_none(),
        TypedFields::RacialTrait(fields) => !fields.alternate,
        TypedFields::Archetype(fields) => fields.replaces.is_empty() && fields.alters.is_empty(),
        TypedFields::ClassOption(fields) => {
            fields.group.is_empty() && fields.replaces.is_empty() && fields.alters.is_empty()
        }
        TypedFields::None => true,
    }
}

fn check_variant_cycles(edges: &BTreeMap<String, Vec<String>>, report: &mut Report) {
    fn visit<'a>(
        node: &'a str,
        edges: &'a BTreeMap<String, Vec<String>>,
        state: &mut BTreeMap<&'a str, bool>,
        path: &mut Vec<&'a str>,
        report: &mut Report,
    ) {
        match state.get(node) {
            Some(true) => {
                // The node is already on the current path: the path from its first occurrence
                // back to itself is the cycle, in order.
                let start = path.iter().position(|id| *id == node).unwrap_or(0);
                let mut cycle: Vec<&str> = path[start..].to_vec();
                cycle.push(node);
                let shown: Vec<String> = cycle.iter().map(|id| format!("`{id}`")).collect();
                report.problems.push(format!(
                    "`variant` relations form a cycle: {}; remove one variant edge",
                    shown.join(" → ")
                ));
                return;
            }
            Some(false) => return,
            None => {}
        }
        state.insert(node, true);
        path.push(node);
        for target in edges.get(node).map(Vec::as_slice).unwrap_or_default() {
            visit(target, edges, state, path, report);
        }
        path.pop();
        state.insert(node, false);
    }
    let mut state = BTreeMap::new();
    let mut path = Vec::new();
    for start in edges.keys() {
        visit(start, edges, &mut state, &mut path, report);
    }
}

fn check_option_groups(corpus: &Corpus, ids: &BTreeSet<&str>, report: &mut Report) {
    let mut used: BTreeMap<&str, usize> = BTreeMap::new();
    for stored in corpus.entries.values() {
        let TypedFields::ClassOption(fields) = &stored.entry.fields else {
            continue;
        };
        let at = format!(
            "{}:{}",
            stored.path.display(),
            line_of(&stored.text, &format!("group = \"{}\"", fields.group))
        );
        if !corpus.option_groups.groups.contains_key(&fields.group) {
            report.problems.push(format!(
                "{at}: class-option group `{}` is not declared in option-groups.toml",
                fields.group
            ));
        }
        *used.entry(fields.group.as_str()).or_default() += 1;
    }
    let text = std::fs::read_to_string(corpus.dir.join("option-groups.toml")).unwrap_or_default();
    for (slug, group) in &corpus.option_groups.groups {
        let line = text
            .lines()
            .position(|line| line.trim() == format!("[groups.{slug}]"))
            .map_or(1, |index| index + 1);
        for class in &group.classes {
            if !ids.contains(class.as_str()) {
                report.problems.push(format!(
                    "option-groups.toml:{line}: group `{slug}` names unknown class `{class}`"
                ));
            }
        }
        if !used.contains_key(slug.as_str()) {
            report.problems.push(format!(
                "option-groups.toml:{line}: group `{slug}` holds no class_option entries"
            ));
        }
    }
}

/// A valid publication suffix is one id segment.
fn valid_suffix(suffix: &str) -> bool {
    !suffix.contains('/') && EntryId::parse(suffix).is_ok()
}

fn publication_suffix<'a>(corpus: &'a Corpus, entry: &RuleEntry) -> Result<&'a str, String> {
    if entry.source_ids.len() != 1 {
        return Err(format!(
            "`{}` needs exactly one publication to take its id suffix",
            entry.id
        ));
    }
    let source = entry.source_ids[0].as_str();
    corpus
        .publications
        .iter()
        .find(|publication| publication.id == source)
        .and_then(|publication| publication.suffix.as_deref())
        .ok_or_else(|| {
            format!(
                "publication `{source}` has no `suffix` in publications.toml, so `{}` cannot take one",
                entry.id
            )
        })
}

fn check_publication_suffixes(corpus: &Corpus, ids: &BTreeSet<&str>, report: &mut Report) {
    let text = std::fs::read_to_string(corpus.dir.join("publications.toml")).unwrap_or_default();
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    let mut known: BTreeSet<String> = ["mythic".to_string(), "stamina".to_string()]
        .into_iter()
        .collect();
    for publication in &corpus.publications {
        let Some(suffix) = publication.suffix.as_deref() else {
            continue;
        };
        let line = text
            .lines()
            .position(|row| row.trim() == format!("suffix = \"{suffix}\""))
            .map_or(1, |index| index + 1);
        if !valid_suffix(suffix) {
            report.problems.push(format!(
                "publications.toml:{line}: publication `{}` has suffix `{suffix}`, which is not one id segment",
                publication.id
            ));
        }
        if let Some(earlier) = seen.insert(suffix, &publication.id) {
            report.problems.push(format!(
                "publications.toml:{line}: publications `{earlier}` and `{}` share the suffix `{suffix}`",
                publication.id
            ));
        }
        known.insert(suffix.to_string());
    }

    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let line = |target: &str| {
            format!(
                "{}:{}",
                stored.path.display(),
                line_of(&stored.text, target)
            )
        };
        for relation in &entry.relations {
            let expected = match (relation.kind, relation.label.as_deref()) {
                (RelationKind::Addon, Some("mythic")) => {
                    // The published carried twins keep their ids and still become the mythic
                    // entries, so they are exempt from nesting under the base.
                    if entry.id.as_str().starts_with("ref/carried/") {
                        continue;
                    }
                    Some("mythic")
                }
                (RelationKind::Addon, Some("stamina")) => Some("stamina"),
                (RelationKind::Addon, Some("changes")) | (RelationKind::Revision, _) => {
                    match publication_suffix(corpus, entry) {
                        Ok(suffix) => Some(suffix),
                        Err(problem) => {
                            report
                                .problems
                                .push(format!("{}: {problem}", line(relation.target.as_str())));
                            None
                        }
                    }
                }
                _ => None,
            };
            let Some(suffix) = expected else { continue };
            let expected_id = format!("{}/{suffix}", relation.target);
            if entry.id.as_str() != expected_id {
                report.problems.push(format!(
                    "{}: a `{}` relation to `{}` needs the id `{expected_id}`, not `{}`",
                    line(relation.target.as_str()),
                    relation.kind.slug(),
                    relation.target,
                    entry.id
                ));
            }
        }
    }

    // The reverse rule: an id that ends in a known suffix and sits under an existing entry must
    // hold the matching edge. A carried id is checked like any other; new carried ids do not
    // end in a suffix.
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let Some((parent, tail)) = entry.id.as_str().rsplit_once('/') else {
            continue;
        };
        if !ids.contains(parent) || !known.contains(tail) {
            continue;
        }
        let matches = entry.relations.iter().any(|relation| {
            if relation.target.as_str() != parent {
                return false;
            }
            match tail {
                "mythic" => {
                    relation.kind == RelationKind::Addon
                        && relation.label.as_deref() == Some("mythic")
                }
                "stamina" => {
                    relation.kind == RelationKind::Addon
                        && relation.label.as_deref() == Some("stamina")
                }
                _ => {
                    (relation.kind == RelationKind::Addon
                        && relation.label.as_deref() == Some("changes"))
                        || relation.kind == RelationKind::Revision
                }
            }
        });
        if !matches {
            report.problems.push(format!(
                "{}: id ends in `/{tail}` under `{parent}`, so it must hold the matching `addon` or `revision` relation",
                stored.path.display()
            ));
        }
    }
}

fn check_book_node(
    corpus: &Corpus,
    node: &BookNode,
    ids: &BTreeSet<&str>,
    keys: &BTreeMap<&str, Vec<(String, String)>>,
    placed: &mut BTreeSet<String>,
    report: &mut Report,
) {
    if let Some(entry) = &node.entry {
        let line = line_of(&corpus.books_text, &format!("\"{entry}\""));
        if !ids.contains(entry.as_str()) {
            report.problems.push(format!(
                "books.toml:{line}: book node `{}` names unknown entry `{entry}`",
                node.title
            ));
        } else {
            match &node.anchor {
                None => {
                    if !placed.insert(entry.to_string()) {
                        report
                            .problems
                            .push(format!("books.toml:{line}: book places `{entry}` twice"));
                    }
                }
                Some(anchor) => {
                    let available = &keys[entry.as_str()];
                    if !available.iter().any(|(key, _)| key == anchor) {
                        report.problems.push(format!(
                            "books.toml:{line}: book node `{}` points at `{entry}#{anchor}`, a heading that does not exist. {}",
                            node.title,
                            anchor_fix(&corpus.entries[entry.as_str()].path, anchor, available)
                        ));
                    }
                }
            }
        }
    }
    for child in &node.children {
        check_book_node(corpus, child, ids, keys, placed, report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{corpus, heading, link, paragraph, rule};
    use cast_iron_schema::{
        BookIndex, BookNode, BookTree, CreatureFields, EntryId, FeatFields, Inline,
    };

    fn creature(id: &str, creature_type: &str) -> RuleEntry {
        let mut entry = rule(id, vec![]);
        entry.kind = EntryKind::Creature;
        entry.fields = TypedFields::Creature(Box::new(CreatureFields {
            creature_type: Some(creature_type.into()),
            ..Default::default()
        }));
        entry
    }

    #[test]
    fn creature_type_must_be_a_pf1e_type() {
        let report = check(&corpus(vec![creature("ref/creature/ok", "magicalBeast")]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);

        let report = check(&corpus(vec![creature("ref/creature/bad", "Magical Beast")]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(problem.starts_with("entries/creature/bad.md:"), "{problem}");
        assert!(
            problem.contains("creature_type `Magical Beast`"),
            "{problem}"
        );
        assert!(problem.contains("magicalBeast"), "{problem}");
    }

    #[test]
    fn a_publication_needs_a_copyright_and_a_notice_or_a_pending_decision() {
        let mut c = corpus(vec![]);
        c.publications[0].copyright = None;
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("publications.toml:1: publication `core-rulebook` has no copyright"),
            "{text}"
        );

        let mut c = corpus(vec![]);
        c.legal.section_15.clear();
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "publication `core-rulebook` (Core Rulebook) is covered by no section_15"
            ),
            "{text}"
        );

        c.legal.notice_pending = vec!["Core Rulebook".into()];
        assert!(check(&c).problems.is_empty());
    }

    #[test]
    fn left_over_upstream_tokens_fail_and_html_entities_do_not() {
        let dirty = rule(
            "ref/rule/a",
            vec![paragraph(vec![Inline::text("&L&Title&FN& and &Q&EM")])],
        );
        let text = check(&corpus(vec![dirty])).problems.join("\n");
        assert!(text.contains("left-over upstream token `&L&`"), "{text}");
        assert!(text.contains("`&FN&`"), "{text}");
        assert!(text.contains("`&Q&`"), "{text}");

        let clean = rule(
            "ref/rule/b",
            vec![paragraph(vec![Inline::text(
                "&times& &quot& &mdash &S &Next1&",
            )])],
        );
        assert!(check(&corpus(vec![clean])).problems.is_empty());
    }

    #[test]
    fn a_left_over_upstream_token_in_a_typed_field_fails() {
        let mut feat = rule("ref/feat/a", vec![]);
        feat.kind = EntryKind::Feat;
        feat.fields = TypedFields::Feat(cast_iron_schema::FeatFields {
            benefit: Some("&FN&gain a bonus".into()),
            ..Default::default()
        });
        let report = check(&corpus(vec![feat]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(problem.starts_with("entries/feat/a.md:9:"), "{problem}");
        assert!(
            problem.contains("left-over upstream token `&FN&`"),
            "{problem}"
        );
        assert!(problem.contains("`[fields]` key `benefit`"), "{problem}");
    }

    #[test]
    fn lowercase_entities_in_a_typed_field_do_not_fail() {
        let mut feat = rule("ref/feat/b", vec![]);
        feat.kind = EntryKind::Feat;
        feat.fields = TypedFields::Feat(cast_iron_schema::FeatFields {
            benefit: Some("&times& &quot&".into()),
            ..Default::default()
        });
        assert!(check(&corpus(vec![feat])).problems.is_empty());
    }

    #[test]
    fn a_token_in_a_title_or_tag_fails() {
        let mut titled = rule("ref/rule/titled", vec![]);
        titled.title = "&FN&Title".into();
        let text = check(&corpus(vec![titled])).problems.join("\n");
        assert!(text.contains("left-over upstream token `&FN&`"), "{text}");
        assert!(text.contains("in the entry title"), "{text}");

        let mut tagged = rule("ref/rule/tagged", vec![]);
        tagged.tags = vec!["&Q&tag".into()];
        let text = check(&corpus(vec![tagged])).problems.join("\n");
        assert!(text.contains("left-over upstream token `&Q&`"), "{text}");
        assert!(text.contains("in the entry tags"), "{text}");

        let mut clean = rule("ref/rule/clean", vec![]);
        clean.title = "&times& Title".into();
        clean.tags = vec!["&quot& tag".into()];
        assert!(check(&corpus(vec![clean])).problems.is_empty());
    }

    #[test]
    fn a_repeated_token_reports_each_occurrence_line() {
        let mut feat = rule("ref/feat/twice", vec![]);
        feat.kind = EntryKind::Feat;
        feat.fields = TypedFields::Feat(FeatFields {
            benefit: Some("&FN& twice".into()),
            special: Some("&FN& twice".into()),
            ..Default::default()
        });
        let c = corpus(vec![feat]);
        let text = &c.entries["ref/feat/twice"].text;
        let expected = [
            line_of(text, "benefit = \"&FN& twice\""),
            line_of(text, "special = \"&FN& twice\""),
        ];
        assert_ne!(
            expected[0], expected[1],
            "the two fields are on separate lines"
        );

        let report = check(&c);
        let reported: Vec<usize> = report
            .problems
            .iter()
            .map(|problem| {
                problem
                    .split(':')
                    .nth(1)
                    .and_then(|line| line.parse().ok())
                    .unwrap_or(0)
            })
            .collect();
        assert_eq!(reported.len(), 2, "{:?}", report.problems);
        assert_eq!(reported, expected, "{:?}", report.problems);
    }

    #[test]
    fn a_clean_corpus_has_no_problems() {
        let a = rule(
            "ref/rule/a",
            vec![paragraph(vec![link("ref/rule/b", Some("sec-deep"), "b")])],
        );
        let b = rule("ref/rule/b", vec![heading(None, "Deep")]);
        let report = check(&corpus(vec![a, b]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn unknown_sources_licences_and_targets_name_the_file() {
        let mut a = rule(
            "ref/rule/a",
            vec![paragraph(vec![link("ref/rule/missing", None, "x")])],
        );
        a.source_ids = vec!["no-such-book".into()];
        a.license_refs = vec!["no-such-licence".into()];
        let report = check(&corpus(vec![a]));
        let text = report.problems.join("\n");
        assert!(text.contains("entries/rule/a.md:"), "{text}");
        assert!(
            text.contains("unknown publication `no-such-book`"),
            "{text}"
        );
        assert!(text.contains("unknown licence `no-such-licence`"), "{text}");
        assert!(
            text.contains("unknown link target `ref/rule/missing`"),
            "{text}"
        );
    }

    #[test]
    fn a_missing_heading_anchor_names_both_files_and_the_fix() {
        let a = rule(
            "ref/rule/a",
            vec![paragraph(vec![link("ref/rule/b", Some("nowhere"), "b")])],
        );
        let b = rule(
            "ref/rule/b",
            vec![
                heading(Some("effects"), "Effects"),
                heading(None, "Other Part"),
            ],
        );
        let report = check(&corpus(vec![a, b]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(problem.starts_with("entries/rule/a.md:"), "{problem}");
        assert!(problem.contains("ref/rule/b#nowhere"), "{problem}");
        assert!(problem.contains("entries/rule/b.md"), "{problem}");
        assert!(problem.contains("`effects` (\"Effects\")"), "{problem}");
        assert!(problem.contains("`sec-other-part`"), "{problem}");
        assert!(problem.contains("add `{#nowhere}`"), "{problem}");
    }

    #[test]
    fn a_book_node_anchor_is_checked_against_the_target() {
        let mut corpus = corpus(vec![rule(
            "ref/rule/b",
            vec![heading(Some("effects"), "Effects")],
        )]);
        corpus.books = BookIndex {
            books: vec![BookTree {
                source: "core-rulebook".into(),
                title: "Core".into(),
                basis: "reviewed".into(),
                review: vec![],
                chapters: vec![BookNode {
                    title: "Chapter".into(),
                    entry: Some(EntryId::parse("ref/rule/b").unwrap()),
                    anchor: Some("gone".into()),
                    review: None,
                    children: vec![],
                }],
            }],
        };
        corpus.books_text = "[[books.chapters]]\nentry = \"ref/rule/b\"\n".into();
        let report = check(&corpus);
        let problem = &report.problems[0];
        assert!(problem.starts_with("books.toml:2:"), "{problem}");
        assert!(problem.contains("entries/rule/b.md"), "{problem}");
        assert!(problem.contains("`effects`"), "{problem}");
    }

    #[test]
    fn typed_fields_must_match_the_entry_kind() {
        let none = TypedFields::None;
        assert!(fields_match(EntryKind::Rule, &none));
        assert!(fields_match(EntryKind::Archetype, &none));
        assert!(!fields_match(EntryKind::Spell, &none));
        let archetype = TypedFields::Archetype(cast_iron_schema::ArchetypeFields::default());
        assert!(fields_match(EntryKind::Archetype, &archetype));
        assert!(!fields_match(EntryKind::ClassOption, &archetype));
        let option = TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
            group: "hex".into(),
            replaces: vec![],
            alters: vec![],
        });
        assert!(fields_match(EntryKind::ClassOption, &option));
        assert!(!fields_match(EntryKind::Archetype, &option));
        assert_eq!(fields_kind(&option), "class_option");
        let class = TypedFields::Class(cast_iron_schema::ClassFields::default());
        assert!(fields_match(EntryKind::Class, &class));
        assert!(fields_match(EntryKind::PrestigeClass, &class));
        assert!(!fields_match(EntryKind::Feat, &class));
        let spell = TypedFields::Spell(cast_iron_schema::SpellFields::default());
        assert!(fields_match(EntryKind::Spell, &spell));
        assert!(!fields_match(EntryKind::Item, &spell));
        assert_eq!(fields_kind(&spell), "spell");
    }

    #[test]
    fn an_entry_kind_that_disagrees_with_its_fields_is_a_problem() {
        let mut a = rule("ref/rule/a", vec![]);
        a.kind = EntryKind::Spell;
        let report = check(&corpus(vec![a]));
        assert!(report.problems[0].contains("does not match"));
    }

    fn typed(mut entry: RuleEntry, kind: EntryKind, fields: TypedFields) -> RuleEntry {
        entry.kind = kind;
        entry.fields = fields;
        entry
    }

    fn spell(id: &str) -> RuleEntry {
        typed(
            rule(id, vec![]),
            EntryKind::Spell,
            TypedFields::Spell(cast_iron_schema::SpellFields::default()),
        )
    }

    fn feat(id: &str, feat_types: &[&str]) -> RuleEntry {
        typed(
            rule(id, vec![]),
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                feat_types: feat_types.iter().map(|kind| kind.to_string()).collect(),
                ..Default::default()
            }),
        )
    }

    fn related(
        mut entry: RuleEntry,
        kind: RelationKind,
        target: &str,
        label: Option<&str>,
    ) -> RuleEntry {
        entry.relations.push(Relation {
            kind,
            target: EntryId::parse(target).unwrap(),
            label: label.map(str::to_string),
        });
        entry
    }

    fn add_publication(corpus: &mut crate::Corpus, id: &str, suffix: Option<&str>) {
        corpus.publications.push(crate::Publication {
            id: id.into(),
            title: id.into(),
            copyright: Some("Core Rulebook, (c) Paizo Inc.".into()),
            category: None,
            publication_date: None,
            suffix: suffix.map(str::to_string),
        });
    }

    fn in_publication(mut entry: RuleEntry, source: &str) -> RuleEntry {
        entry.source_ids = vec![source.into()];
        entry
    }

    #[test]
    fn relation_labels_are_closed_per_kind() {
        let base = spell("ref/spell/base");
        let a = related(
            spell("ref/spell/a"),
            RelationKind::Prerequisite,
            "ref/spell/base",
            Some("alternate"),
        );
        let b = related(
            spell("ref/spell/b"),
            RelationKind::Addon,
            "ref/spell/base",
            None,
        );
        let c = related(
            spell("ref/spell/c"),
            RelationKind::Revision,
            "ref/spell/base",
            Some("or"),
        );
        let d = related(
            spell("ref/spell/d"),
            RelationKind::Variant,
            "ref/spell/base",
            Some("mythic"),
        );
        let text = check(&corpus(vec![base, a, b, c, d])).problems.join("\n");
        for expected in [
            "`alternate` is not a valid label on a `prerequisite` relation",
            "an `addon` relation needs exactly one of `mythic`, `stamina` or `changes`",
            "`or` is not a valid label on a `revision` relation",
            "`mythic` is not a valid label on a `variant` relation",
        ] {
            assert!(text.contains(expected), "{expected} missing from {text}");
        }
    }

    #[test]
    fn a_variant_goes_from_spell_to_spell_and_only_once() {
        let base = spell("ref/spell/base");
        let rule_variant = related(
            rule("ref/rule/a", vec![]),
            RelationKind::Variant,
            "ref/spell/base",
            None,
        );
        let twice = related(
            related(
                spell("ref/spell/b"),
                RelationKind::Variant,
                "ref/spell/base",
                None,
            ),
            RelationKind::Variant,
            "ref/spell/base",
            None,
        );
        let text = check(&corpus(vec![base, rule_variant, twice]))
            .problems
            .join("\n");
        assert!(
            text.contains("a `variant` relation must go from a spell to a spell"),
            "{text}"
        );
        assert!(
            text.contains("holds 2 `variant` relations; at most one is allowed"),
            "{text}"
        );
    }

    #[test]
    fn a_variant_cycle_is_an_error() {
        let a = related(
            spell("ref/spell/a"),
            RelationKind::Variant,
            "ref/spell/b",
            None,
        );
        let b = related(
            spell("ref/spell/b"),
            RelationKind::Variant,
            "ref/spell/c",
            None,
        );
        let c = related(
            spell("ref/spell/c"),
            RelationKind::Variant,
            "ref/spell/a",
            None,
        );
        let text = check(&corpus(vec![a, b, c])).problems.join("\n");
        assert!(
            text.contains(
                "`variant` relations form a cycle: `ref/spell/a` → `ref/spell/b` → `ref/spell/c` → `ref/spell/a`; remove one variant edge"
            ),
            "{text}"
        );
    }

    #[test]
    fn a_revision_target_must_be_a_non_revision_of_the_same_kind() {
        let base = feat("ref/feat/base", &[]);
        let first = in_publication(
            related(
                feat("ref/feat/base/eitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let second = in_publication(
            related(
                feat("ref/feat/base/two", &[]),
                RelationKind::Revision,
                "ref/feat/base/eitr",
                None,
            ),
            "eitr",
        );
        let wrong_kind = in_publication(
            related(
                spell("ref/spell/base/eitr"),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let mut c = corpus(vec![base, first, second, wrong_kind]);
        add_publication(&mut c, "eitr", Some("eitr"));
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "a `revision` cannot point at `ref/feat/base/eitr`, which is itself a revision"
            ),
            "{text}"
        );
        assert!(
            text.contains("a `revision` must keep the target's kind"),
            "{text}"
        );
    }

    #[test]
    fn a_revision_must_come_from_another_book_and_only_once_per_book() {
        let base = feat("ref/feat/base", &[]);
        let same_book = related(
            feat("ref/feat/base/eitr", &[]),
            RelationKind::Revision,
            "ref/feat/base",
            None,
        );
        let text = check(&corpus(vec![base.clone(), same_book]))
            .problems
            .join("\n");
        assert!(text.contains("shares a book with it"), "{text}");

        let one = in_publication(
            related(
                feat("ref/feat/base/eitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let two = in_publication(
            related(
                feat("ref/feat/base/teitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let mut c = corpus(vec![base, one, two]);
        add_publication(&mut c, "eitr", Some("eitr"));
        let text = check(&c).problems.join("\n");
        assert!(text.contains("is a second revision"), "{text}");
    }

    #[test]
    fn a_revision_id_may_not_be_linked_or_named_as_a_prerequisite() {
        let base = feat("ref/feat/base", &[]);
        let revision = in_publication(
            related(
                feat("ref/feat/base/eitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let linker = rule(
            "ref/rule/linker",
            vec![paragraph(vec![link(
                "ref/feat/base/eitr",
                None,
                "revision",
            )])],
        );
        let prereq = typed(
            rule("ref/feat/after", vec![]),
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                prerequisite_feats: vec![EntryId::parse("ref/feat/base/eitr").unwrap()],
                ..Default::default()
            }),
        );
        let mut c = corpus(vec![base, revision, linker, prereq]);
        add_publication(&mut c, "eitr", Some("eitr"));
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("the body links `ref/feat/base/eitr`, which is a revision"),
            "{text}"
        );
        assert!(
            text.contains("`prerequisite_feats` names `ref/feat/base/eitr`, which is a revision"),
            "{text}"
        );
    }

    #[test]
    fn removed_and_merged_revisions_have_empty_fields_and_a_merged_link() {
        let base = feat("ref/feat/base", &[]);
        let removed = in_publication(
            related(
                typed(
                    rule("ref/feat/base/eitr", vec![]),
                    EntryKind::Feat,
                    TypedFields::Feat(FeatFields {
                        benefit: Some("not empty".into()),
                        ..Default::default()
                    }),
                ),
                RelationKind::Revision,
                "ref/feat/base",
                Some("removed"),
            ),
            "eitr",
        );
        let merged = in_publication(
            related(
                feat("ref/feat/merged/eitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                Some("merged"),
            ),
            "eitr",
        );
        let target = feat("ref/feat/merged", &[]);
        let mut c = corpus(vec![base, removed, merged, target]);
        add_publication(&mut c, "eitr", Some("eitr"));
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("a `removed` revision must have empty `[fields]`"),
            "{text}"
        );
        assert!(
            text.contains("a `merged` revision must link the feat it merges into"),
            "{text}"
        );
    }

    #[test]
    fn a_mythic_add_on_must_be_a_mythic_feat() {
        let base = feat("ref/feat/base", &[]);
        let not_mythic = related(
            feat("ref/feat/base/mythic", &["Combat"]),
            RelationKind::Addon,
            "ref/feat/base",
            Some("mythic"),
        );
        let text = check(&corpus(vec![base, not_mythic])).problems.join("\n");
        assert!(
            text.contains("a `mythic` add-on must be a feat whose `feat_types` include `Mythic`"),
            "{text}"
        );
    }

    #[test]
    fn a_non_feat_mythic_holder_passes() {
        // The mythic-spell add-ons of slice 3 are `rule` entries under the spell they extend.
        let base = spell("ref/spell/heal");
        let holder = related(
            rule("ref/spell/heal/mythic", vec![]),
            RelationKind::Addon,
            "ref/spell/heal",
            Some("mythic"),
        );
        let report = check(&corpus(vec![base, holder]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn a_carried_mythic_twin_is_exempt_from_the_nested_id() {
        let base = feat("ref/feat/power-attack", &[]);
        let twin = typed(
            rule(
                "ref/carried/mythic-adventures/feats/power-attack/mythic",
                vec![],
            ),
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                feat_types: vec!["Mythic".into()],
                ..Default::default()
            }),
        );
        let twin = related(
            twin,
            RelationKind::Addon,
            "ref/feat/power-attack",
            Some("mythic"),
        );
        let report = check(&corpus(vec![base.clone(), twin]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);

        let wrong = related(
            feat("ref/feat/elsewhere", &["Mythic"]),
            RelationKind::Addon,
            "ref/feat/power-attack",
            Some("mythic"),
        );
        let text = check(&corpus(vec![base, wrong])).problems.join("\n");
        assert!(
            text.contains("needs the id `ref/feat/power-attack/mythic`, not `ref/feat/elsewhere`"),
            "{text}"
        );
    }

    fn class_option(id: &str, group: &str) -> RuleEntry {
        typed(
            rule(id, vec![]),
            EntryKind::ClassOption,
            TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
                group: group.into(),
                replaces: vec![],
                alters: vec![],
            }),
        )
    }

    #[test]
    fn a_class_option_group_must_exist_resolve_its_classes_and_hold_an_entry() {
        let entry = class_option("ref/arcanedisc/a", "missing");
        let text = check(&corpus(vec![entry])).problems.join("\n");
        assert!(
            text.contains("class-option group `missing` is not declared in option-groups.toml"),
            "{text}"
        );

        let mut c = corpus(vec![class_option("ref/arcanedisc/a", "discoveries")]);
        c.option_groups.groups.insert(
            "discoveries".into(),
            crate::OptionGroupDef {
                label: "Discoveries".into(),
                singular: "Discovery".into(),
                classes: vec!["ref/class/ghost".into()],
            },
        );
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("group `discoveries` names unknown class `ref/class/ghost`"),
            "{text}"
        );

        let mut c = corpus(vec![]);
        c.option_groups.groups.insert(
            "empty".into(),
            crate::OptionGroupDef {
                label: "Empty".into(),
                singular: "Empty".into(),
                classes: vec![],
            },
        );
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("group `empty` holds no class_option entries"),
            "{text}"
        );
    }

    #[test]
    fn a_path_that_does_not_match_its_id_is_an_error() {
        let mut c = corpus(vec![rule("ref/rule/a", vec![])]);
        c.entries.get_mut("ref/rule/a").unwrap().path =
            std::path::PathBuf::from("entries/rule/wrong.md");
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "entry id `ref/rule/a` belongs at `entries/rule/a.md`, not `entries/rule/wrong.md`"
            ),
            "{text}"
        );
    }

    #[test]
    fn an_add_on_and_a_revision_take_the_expected_suffix() {
        let base = feat("ref/feat/base", &[]);
        let good = related(
            feat("ref/feat/base/mythic", &["Mythic"]),
            RelationKind::Addon,
            "ref/feat/base",
            Some("mythic"),
        );
        let bad = related(
            feat("ref/feat/wrong", &["Mythic"]),
            RelationKind::Addon,
            "ref/feat/base",
            Some("mythic"),
        );
        let text = check(&corpus(vec![base, good, bad])).problems.join("\n");
        assert!(
            text.contains("needs the id `ref/feat/base/mythic`, not `ref/feat/wrong`"),
            "{text}"
        );

        let base = feat("ref/feat/base", &[]);
        let changed = related(
            feat("ref/feat/base/teitr", &[]),
            RelationKind::Addon,
            "ref/feat/base",
            Some("changes"),
        );
        let text = check(&corpus(vec![base, changed])).problems.join("\n");
        assert!(
            text.contains("publication `core-rulebook` has no `suffix` in publications.toml"),
            "{text}"
        );

        let base = feat("ref/feat/base", &[]);
        let changed = in_publication(
            related(
                feat("ref/feat/base/teitr", &[]),
                RelationKind::Addon,
                "ref/feat/base",
                Some("changes"),
            ),
            "eitr",
        );
        let mut c = corpus(vec![base.clone(), changed]);
        add_publication(&mut c, "eitr", Some("teitr"));
        assert!(check(&c).problems.is_empty(), "{:?}", check(&c).problems);

        let mut c = corpus(vec![base, feat("ref/feat/other", &[])]);
        add_publication(&mut c, "eitr", Some("teitr"));
        add_publication(&mut c, "other", Some("teitr"));
        let text = check(&c).problems.join("\n");
        assert!(text.contains("share the suffix `teitr`"), "{text}");
    }

    #[test]
    fn a_publication_suffix_must_be_one_id_segment() {
        let mut c = corpus(vec![]);
        add_publication(&mut c, "eitr", Some("Bad Suffix"));
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("has suffix `Bad Suffix`, which is not one id segment"),
            "{text}"
        );
    }

    #[test]
    fn an_id_under_an_entry_that_ends_in_a_suffix_must_hold_the_edge() {
        let base = feat("ref/feat/base", &[]);
        let orphan = feat("ref/feat/base/mythic", &["Mythic"]);
        let text = check(&corpus(vec![base, orphan])).problems.join("\n");
        assert!(
            text.contains("id ends in `/mythic` under `ref/feat/base`"),
            "{text}"
        );
    }

    #[test]
    fn replaces_and_alters_drift_against_the_printed_lines() {
        let archetype = typed(
            rule(
                "ref/arc/example",
                vec![paragraph(vec![
                    Inline::Strong {
                        c: vec![Inline::text("Replaces:")],
                    },
                    Inline::text(" Uncanny dodge."),
                ])],
            ),
            EntryKind::Archetype,
            TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
                replaces: vec!["Trap sense".into()],
                alters: vec![],
            }),
        );
        let text = check(&corpus(vec![archetype])).problems.join("\n");
        assert!(
            text.contains(
                "[fields] replaces = \"Trap sense\" disagrees with the `**Replaces:**` line"
            ),
            "{text}"
        );

        let option = typed(
            rule(
                "ref/arcanedisc/example",
                vec![paragraph(vec![
                    Inline::Strong {
                        c: vec![Inline::text("Replaces:")],
                    },
                    Inline::text(" Arcane bond."),
                ])],
            ),
            EntryKind::ClassOption,
            TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
                group: "discoveries".into(),
                replaces: vec!["Arcane school".into()],
                alters: vec![],
            }),
        );
        let mut c = corpus(vec![option]);
        c.option_groups.groups.insert(
            "discoveries".into(),
            crate::OptionGroupDef {
                label: "Discoveries".into(),
                singular: "Discovery".into(),
                classes: vec![],
            },
        );
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "[fields] replaces = \"Arcane school\" disagrees with the `**Replaces:**` line"
            ),
            "{text}"
        );
    }
}
