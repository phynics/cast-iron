//! Planning (ids, duplicates, collisions) and rendering (entries and bodies).

use crate::config::IdOverrides;
use crate::fields::{self, Extracted};
use crate::html::{self, Env};
use crate::model::Node;
use crate::select::Survey;
use crate::slug::{slug_segment, url_path, url_slug};
use cast_iron_schema::{
    Block, EntryId, EntryKind, EntryReference, Inline, LinkCoverage, ParagraphStyle, Provenance,
    Rejection, RuleEntry, StatRow, TypedFields,
};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const LICENSE_OGL: &str = "ogl-1.0a";

pub struct RawTyped {
    pub rel: String,
    pub kind: EntryKind,
    pub node: Node,
}

pub struct RawRules {
    pub rel: String,
    pub node: Node,
}

pub struct TypedInput {
    pub rel: String,
    pub kind: EntryKind,
    pub node: Node,
    pub id: EntryId,
    pub title: String,
    pub value: Value,
    pub extracted: Extracted,
}

pub enum ChildPlan {
    /// The child is a selected typed record: link, never duplicate.
    Typed { url: String, idx: usize },
    /// The child becomes its own Rule entry.
    Entry { id: EntryId, idx: usize },
    /// The child repeats an entry that an earlier child already created.
    Dup { id: EntryId, idx: usize },
    /// The child has no name of its own, so it renders inside the parent.
    Inline { idx: usize },
}

pub struct RulesPlan {
    pub rel: String,
    pub node: Node,
    pub id: EntryId,
    pub title: String,
    pub children: Vec<ChildPlan>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DedupeStats {
    pub typed_identical: u32,
    pub typed_conflicting: u32,
    pub nested_replaced_identical: u32,
    pub nested_replaced_conflicting: u32,
    pub rules_children_identical: u32,
    pub rules_children_conflicting: u32,
}

pub struct Plan<'a> {
    pub dataset: String,
    pub typed: Vec<TypedInput>,
    pub typed_by_url: BTreeMap<String, usize>,
    pub rules: Vec<RulesPlan>,
    pub survey: &'a Survey,
    pub overrides: &'a IdOverrides,
    pub rejected: Vec<Rejection>,
    pub notes: Vec<String>,
    pub dedupe: DedupeStats,
    pub extra_prov: BTreeMap<EntryId, BTreeSet<(String, String)>>,
}

pub fn id_for(overrides: &IdOverrides, url: &str) -> Result<EntryId, String> {
    if let Some(explicit) = overrides.ids.get(url) {
        return EntryId::parse(explicit.clone())
            .map_err(|e| format!("id-overrides.toml value for {url}: {e}"));
    }
    let slug = url_slug(url).ok_or_else(|| format!("url {url:?} has no usable path"))?;
    EntryId::parse(slug).map_err(|e| e.to_string())
}

struct Claims(BTreeMap<String, String>);

impl Claims {
    fn claim(&mut self, id: &EntryId, who: String, errors: &mut Vec<String>) -> bool {
        match self.0.get(id.as_str()) {
            Some(other) if *other != who => {
                let (a, b) = if *other < who {
                    (other, &who)
                } else {
                    (&who, other)
                };
                errors.push(format!(
                    "id collision on `{id}`: both {a} and {b} slugify to it. Add one of the urls to [ids] in id-overrides.toml"
                ));
                false
            }
            _ => {
                self.0.insert(id.to_string(), who);
                true
            }
        }
    }
}

/// Builds the immutable plan. Fatal problems, such as id collisions, come back as messages.
pub fn plan<'a>(
    typed: Vec<RawTyped>,
    rules: Vec<RawRules>,
    survey: &'a Survey,
    overrides: &'a IdOverrides,
    dataset: &str,
) -> Result<Plan<'a>, Vec<String>> {
    let mut errors = Vec::new();
    let mut rejected = Vec::new();
    let mut notes = Vec::new();
    let mut dedupe = DedupeStats::default();
    let mut extra_prov: BTreeMap<EntryId, BTreeSet<(String, String)>> = BTreeMap::new();

    let mut typed = typed;
    typed.sort_by(|a, b| a.rel.cmp(&b.rel));
    let mut inputs: Vec<TypedInput> = Vec::new();
    let mut by_url: BTreeMap<String, usize> = BTreeMap::new();
    let mut claims = Claims(BTreeMap::new());
    for raw in typed {
        let url = raw.node.url.clone().unwrap_or_default();
        let id = match id_for(overrides, &url) {
            Ok(id) => id,
            Err(reason) => {
                rejected.push(Rejection {
                    path: raw.rel,
                    reason,
                });
                continue;
            }
        };
        let extracted = match fields::extract(raw.kind, &raw.node, &raw.rel) {
            Ok(e) => e,
            Err(reason) => {
                rejected.push(Rejection {
                    path: raw.rel,
                    reason,
                });
                continue;
            }
        };
        // Categories may change; public entry IDs remain URL-derived.
        let kind = match (&extracted.fields, raw.kind) {
            (TypedFields::Class(fields), EntryKind::Class)
                if fields.class_type.as_deref() == Some("prestige") =>
            {
                EntryKind::PrestigeClass
            }
            _ => raw.kind,
        };
        let value = raw.node.to_value();
        if let Some(&first) = by_url.get(&url) {
            let canonical = &mut inputs[first];
            if canonical.value == value {
                dedupe.typed_identical += 1;
            } else {
                dedupe.typed_conflicting += 1;
                notes.push(format!(
                    "conflicting duplicate {url}: {} and {} differ; kept {} and recorded both as provenance",
                    canonical.rel, raw.rel, canonical.rel
                ));
                extra_prov
                    .entry(canonical.id.clone())
                    .or_default()
                    .insert((url.clone(), raw.rel.clone()));
                // A construct can also have an item record at the same URL.
                // Keep the canonical category while retaining supplemental
                // construction metadata instead of discarding disjoint fields.
                if canonical.kind == EntryKind::Creature && kind == EntryKind::Item {
                    for (key, value) in &raw.node.rest {
                        canonical
                            .node
                            .rest
                            .entry(key.clone())
                            .or_insert_with(|| value.clone());
                    }
                    canonical.value = canonical.node.to_value();
                }
            }
            continue;
        }
        claims.claim(&id, format!("{url} ({})", raw.rel), &mut errors);
        by_url.insert(url, inputs.len());
        inputs.push(TypedInput {
            title: html::plain_text(raw.node.title()),
            rel: raw.rel,
            kind,
            node: raw.node,
            id,
            value,
            extracted,
        });
    }

    let mut rules = rules;
    rules.sort_by(|a, b| a.rel.cmp(&b.rel));
    let mut rule_plans = Vec::new();
    let mut claimed_children: BTreeMap<String, (EntryId, Value)> = BTreeMap::new();
    for raw in rules {
        let url = raw.node.url.clone().unwrap_or_default();
        let id = match id_for(overrides, &url) {
            Ok(id) => id,
            Err(reason) => {
                rejected.push(Rejection {
                    path: raw.rel,
                    reason,
                });
                continue;
            }
        };
        claims.claim(&id, format!("{url} ({})", raw.rel), &mut errors);
        let mut children = Vec::new();
        for (idx, child) in raw.node.sections.iter().enumerate() {
            let curl = child.url.clone();
            if let Some(curl) = &curl
                && by_url.contains_key(curl)
            {
                children.push(ChildPlan::Typed {
                    url: curl.clone(),
                    idx,
                });
                continue;
            }
            let named = child.name.as_deref().is_some_and(|n| !n.trim().is_empty());
            if !named || child.kind.as_deref() == Some("link") {
                children.push(ChildPlan::Inline { idx });
                continue;
            }
            if let Some(curl) = &curl
                && let Some((existing, value)) = claimed_children.get(curl)
            {
                if *value == child.to_value() {
                    dedupe.rules_children_identical += 1;
                } else {
                    dedupe.rules_children_conflicting += 1;
                    notes.push(format!(
                        "conflicting duplicate section {curl}: {} repeats it with a different body",
                        raw.rel
                    ));
                    extra_prov
                        .entry(existing.clone())
                        .or_default()
                        .insert((curl.clone(), raw.rel.clone()));
                }
                children.push(ChildPlan::Dup {
                    id: existing.clone(),
                    idx,
                });
                continue;
            }
            let cid = match curl.as_deref() {
                Some(u) => match id_for(overrides, u) {
                    Ok(id) => id,
                    Err(reason) => {
                        rejected.push(Rejection {
                            path: format!("{}#{u}", raw.rel),
                            reason,
                        });
                        continue;
                    }
                },
                None => {
                    let base = format!("{}/{}", id, slug_segment(child.title()));
                    let mut candidate = base.clone();
                    if claims.0.contains_key(&candidate) {
                        candidate = format!("{base}-{}", idx + 1);
                    }
                    match EntryId::parse(candidate) {
                        Ok(id) => id,
                        Err(e) => {
                            rejected.push(Rejection {
                                path: raw.rel.clone(),
                                reason: e.to_string(),
                            });
                            continue;
                        }
                    }
                }
            };
            let who = match &curl {
                Some(u) => format!("{u} ({})", raw.rel),
                None => format!("{}#{idx} ({})", raw.rel, child.title()),
            };
            claims.claim(&cid, who, &mut errors);
            if let Some(curl) = curl {
                claimed_children.insert(curl, (cid.clone(), child.to_value()));
            }
            children.push(ChildPlan::Entry { id: cid, idx });
        }
        rule_plans.push(RulesPlan {
            title: html::plain_text(raw.node.title()),
            rel: raw.rel,
            node: raw.node,
            id,
            children,
        });
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }
    rejected.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Plan {
        dataset: dataset.to_string(),
        typed: inputs,
        typed_by_url: by_url,
        rules: rule_plans,
        survey,
        overrides,
        rejected,
        notes,
        dedupe,
        extra_prov,
    })
}

// ---------------------------------------------------------------------------------------------
// Rendering

pub struct Built {
    pub entries: Vec<RuleEntry>,
    pub stripped: BTreeMap<String, u32>,
    pub links: LinkCoverage,
    pub notes: Vec<String>,
    pub dedupe: DedupeStats,
    /// Source slug to every raw `source` value seen for it.
    pub source_titles: BTreeMap<String, BTreeSet<String>>,
}

struct State<'a> {
    index: BTreeMap<String, (EntryId, Option<String>)>,
    stripped: BTreeMap<String, u32>,
    links: LinkCoverage,
    notes: Vec<String>,
    nested_identical: u32,
    nested_conflicting: u32,
    extra_prov: BTreeMap<EntryId, BTreeSet<(String, String)>>,
    source_titles: BTreeMap<String, BTreeSet<String>>,
    survey: &'a Survey,
    overrides: &'a IdOverrides,
    cur_id: Option<EntryId>,
    cur_url: String,
    cur_path: String,
    cur_anchors: BTreeSet<String>,
    cur_links: BTreeSet<EntryReference>,
}

impl Env for State<'_> {
    fn link(&mut self, href: &str) -> Option<(EntryId, Option<String>)> {
        let href = href.trim();
        if href.is_empty() {
            self.links.broken += 1;
            return None;
        }
        let href = self
            .overrides
            .equivalent_urls
            .get(href)
            .map(String::as_str)
            .unwrap_or(href);
        let hit = self.index.get(href).cloned().or_else(|| {
            let prefix = format!("{href}/");
            let mut deeper = self
                .index
                .range(prefix.clone()..)
                .take_while(|(k, _)| k.starts_with(&prefix));
            match (deeper.next(), deeper.next()) {
                (Some((_, target)), None) => Some(target.clone()),
                _ => None,
            }
        });
        match hit {
            Some((id, anchor)) => {
                self.links.resolved += 1;
                self.cur_links.insert(EntryReference {
                    target: id.clone(),
                    anchor: anchor.clone(),
                });
                Some((id, anchor))
            }
            None => {
                if self.survey.has_url_or_descendant(href) {
                    self.links.out_of_selection += 1;
                } else {
                    self.links.broken += 1;
                }
                None
            }
        }
    }

    fn stripped(&mut self, key: &str) {
        *self.stripped.entry(key.to_string()).or_default() += 1;
    }
}

struct Draft<'a> {
    id: &'a EntryId,
    kind: EntryKind,
    title: String,
    record: String,
    path: String,
    parent: Option<EntryId>,
    section: Vec<String>,
    body: Vec<Block>,
    extracted: Option<&'a Extracted>,
}

pub struct Ctx<'p> {
    plan: &'p Plan<'p>,
    st: State<'p>,
}

/// Renders every entry twice: the first pass fills the url index, the second resolves links
/// against the complete index and produces the output.
pub fn build<'p>(plan: &'p Plan<'p>) -> Built {
    let mut ctx = Ctx {
        plan,
        st: State {
            index: BTreeMap::new(),
            stripped: BTreeMap::new(),
            links: LinkCoverage::default(),
            notes: Vec::new(),
            nested_identical: 0,
            nested_conflicting: 0,
            extra_prov: BTreeMap::new(),
            source_titles: BTreeMap::new(),
            survey: plan.survey,
            overrides: plan.overrides,
            cur_id: None,
            cur_url: String::new(),
            cur_path: String::new(),
            cur_anchors: BTreeSet::new(),
            cur_links: BTreeSet::new(),
        },
    };
    for t in &plan.typed {
        let url = t.node.url.clone().unwrap_or_default();
        ctx.st.index.entry(url).or_insert((t.id.clone(), None));
    }
    for r in &plan.rules {
        let url = r.node.url.clone().unwrap_or_default();
        ctx.st.index.entry(url).or_insert((r.id.clone(), None));
        for child in &r.children {
            if let ChildPlan::Entry { id, idx } = child
                && let Some(u) = &r.node.sections[*idx].url
            {
                ctx.st.index.entry(u.clone()).or_insert((id.clone(), None));
            }
        }
    }
    let _ = ctx.run();
    ctx.st.stripped.clear();
    ctx.st.links = LinkCoverage::default();
    ctx.st.notes.clear();
    ctx.st.nested_identical = 0;
    ctx.st.nested_conflicting = 0;
    ctx.st.extra_prov.clear();
    ctx.st.source_titles.clear();
    let mut entries = ctx.run();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    let mut extra = plan.extra_prov.clone();
    for (id, set) in std::mem::take(&mut ctx.st.extra_prov) {
        extra.entry(id).or_default().extend(set);
    }
    for entry in &mut entries {
        if let Some(set) = extra.get(&entry.id) {
            for (record, path) in set {
                let p = Provenance {
                    dataset: plan.dataset.clone(),
                    record: record.clone(),
                    path: path.clone(),
                };
                if !entry.provenance.contains(&p) {
                    entry.provenance.push(p);
                }
            }
        }
    }
    let mut dedupe = plan.dedupe.clone();
    dedupe.nested_replaced_identical = ctx.st.nested_identical;
    dedupe.nested_replaced_conflicting = ctx.st.nested_conflicting;
    let mut notes = plan.notes.clone();
    notes.extend(std::mem::take(&mut ctx.st.notes));
    Built {
        entries,
        stripped: ctx.st.stripped,
        links: ctx.st.links,
        notes,
        dedupe,
        source_titles: ctx.st.source_titles,
    }
}

fn humanize(key: &str) -> String {
    key.replace('_', " ")
        .split(' ')
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map_or_else(String::new, |f| {
                f.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn breadcrumb(url: &str) -> Vec<String> {
    let Some(path) = url_path(url) else {
        return Vec::new();
    };
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 2 {
        return Vec::new();
    }
    parts[1..parts.len() - 1]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn yes_no(value: Option<&Value>) -> Option<&'static str> {
    value
        .and_then(Value::as_bool)
        .map(|b| if b { "Yes" } else { "No" })
}

impl<'p> Ctx<'p> {
    fn run(&mut self) -> Vec<RuleEntry> {
        let plan = self.plan;
        let mut entries = Vec::new();
        for t in &plan.typed {
            entries.push(self.typed_entry(t));
        }
        for r in &plan.rules {
            self.rules_entries(r, &mut entries);
        }
        entries
    }

    fn begin(&mut self, id: &EntryId, url: &str, path: &str) {
        self.st.cur_id = Some(id.clone());
        self.st.cur_url = url.to_string();
        self.st.cur_path = path.to_string();
        self.st.cur_anchors.clear();
        self.st.cur_links.clear();
    }

    fn inlines(&mut self, raw: &str) -> Vec<Inline> {
        html::to_inlines(raw, &mut self.st)
    }

    fn push_row(&mut self, rows: &mut Vec<StatRow>, label: &str, raw: Option<&str>) {
        if let Some(raw) = raw {
            let value = self.inlines(raw);
            if !value.is_empty() {
                rows.push(StatRow {
                    label: label.to_string(),
                    value,
                });
            }
        }
    }

    fn ignore(&mut self, node: &Node, keys: &[&str]) {
        for key in keys {
            if node.rest.contains_key(*key) {
                self.st.stripped(&format!("ignored_field:{key}"));
            }
        }
    }

    fn unique_anchor(&mut self, name: &str) -> String {
        let base = {
            let s = slug_segment(&html::plain_text(name));
            if s.is_empty() {
                "section".to_string()
            } else {
                s
            }
        };
        let mut candidate = base.clone();
        let mut n = 2;
        while self.st.cur_anchors.contains(&candidate) {
            candidate = format!("{base}-{n}");
            n += 1;
        }
        self.st.cur_anchors.insert(candidate.clone());
        candidate
    }

    fn register(&mut self, url: &str, anchor: Option<String>) {
        if let Some(id) = self.st.cur_id.clone() {
            self.st.index.entry(url.to_string()).or_insert((id, anchor));
        }
    }

    fn extra_rows(&mut self, rest: &Map<String, Value>, consumed: &[&str]) -> Vec<StatRow> {
        let mut rows = Vec::new();
        for (key, value) in rest {
            if !consumed.contains(&key.as_str()) {
                self.flatten_rows(&humanize(key), value, &mut rows);
            }
        }
        rows
    }

    fn flatten_rows(&mut self, label: &str, value: &Value, rows: &mut Vec<StatRow>) {
        match value {
            Value::String(s) => self.push_row(rows, label, Some(s)),
            Value::Number(n) => self.push_row(rows, label, Some(&n.to_string())),
            Value::Bool(b) => self.push_row(rows, label, Some(if *b { "Yes" } else { "No" })),
            Value::Array(items) => {
                for item in items {
                    self.flatten_rows(label, item, rows);
                }
            }
            Value::Object(map) => {
                for (key, inner) in map {
                    match inner {
                        Value::Object(deep) => {
                            for (k2, v2) in deep {
                                let l = if key == "null" {
                                    humanize(k2)
                                } else {
                                    format!("{key}: {k2}")
                                };
                                self.flatten_rows(&l, v2, rows);
                            }
                        }
                        _ => self.flatten_rows(&humanize(key), inner, rows),
                    }
                }
            }
            Value::Null => {}
        }
    }

    fn description_block(&mut self, node: &Node, out: &mut Vec<Block>) {
        if let Some(d) = &node.description {
            let c = self.inlines(d);
            if !c.is_empty() {
                out.push(Block::Paragraph {
                    style: ParagraphStyle::Normal,
                    content: vec![Inline::Em { c }],
                });
            }
        }
    }

    fn body_blocks(&mut self, node: &Node, out: &mut Vec<Block>) {
        if let Some(body) = &node.body {
            out.extend(html::to_blocks(body, &mut self.st));
        }
    }

    /// A rules tree nests a copy of a typed record. The typed record wins; a copy that differs
    /// is reported and its location is kept as extra provenance.
    fn note_nested_copy(&mut self, node: &Node, typed: &TypedInput) {
        let url = node.url.as_deref().unwrap_or_default();
        if typed.value == node.to_value() {
            self.st.nested_identical += 1;
        } else {
            self.st.nested_conflicting += 1;
            self.st.notes.push(format!(
                "conflicting duplicate {url}: the copy nested in {} differs from {}; linked to the typed entry and kept both as provenance",
                self.st.cur_path, typed.rel
            ));
            let path = self.st.cur_path.clone();
            self.st
                .extra_prov
                .entry(typed.id.clone())
                .or_default()
                .insert((url.to_string(), path));
        }
    }

    fn render_children(
        &mut self,
        children: &[Node],
        depth: usize,
        enclosing: Option<&str>,
        out: &mut Vec<Block>,
    ) {
        for child in children {
            self.render_node(child, depth, enclosing, out);
        }
    }

    fn render_node(
        &mut self,
        node: &Node,
        depth: usize,
        enclosing: Option<&str>,
        out: &mut Vec<Block>,
    ) {
        let plan = self.plan;
        if let Some(url) = node.url.as_deref()
            && url != self.st.cur_url
            && let Some(&ti) = plan.typed_by_url.get(url)
        {
            let typed = &plan.typed[ti];
            self.note_nested_copy(node, typed);
            self.st.cur_links.insert(EntryReference {
                target: typed.id.clone(),
                anchor: None,
            });
            out.push(Block::EntryLink {
                target: typed.id.clone(),
                title: typed.title.clone(),
            });
            return;
        }
        if node.kind.as_deref() == Some("link") {
            if let Some(url) = node.url.as_deref() {
                self.register(url, enclosing.map(str::to_string));
            }
            for raw in [node.name.as_deref(), node.body.as_deref()]
                .into_iter()
                .flatten()
            {
                let content = self.inlines(raw);
                if !content.is_empty() {
                    out.push(Block::Paragraph {
                        style: ParagraphStyle::Normal,
                        content,
                    });
                }
            }
            return;
        }
        let name = node.name.as_deref().filter(|n| !n.trim().is_empty());
        let anchor = name.map(|n| self.unique_anchor(n));
        if let Some(url) = node.url.as_deref() {
            let target = anchor.clone().or_else(|| enclosing.map(str::to_string));
            self.register(url, target);
        }
        if let (Some(name), Some(anchor)) = (name, &anchor) {
            let content = self.inlines(name);
            if !content.is_empty() {
                out.push(Block::Heading {
                    level: (2 + depth).min(6) as u8,
                    anchor: Some(anchor.clone()),
                    content,
                });
            }
        }
        self.description_block(node, out);
        let rows = self.extra_rows(&node.rest, &[]);
        if !rows.is_empty() {
            out.push(Block::StatBlock { title: None, rows });
        }
        self.body_blocks(node, out);
        let next = anchor.as_deref().or(enclosing);
        self.render_children(&node.sections, depth + 1, next, out);
    }

    fn make_entry(&mut self, node: &Node, d: Draft<'_>) -> RuleEntry {
        let Draft {
            id,
            kind,
            title,
            record,
            path,
            parent,
            section,
            body,
            extracted,
        } = d;
        let source = node.source.clone().unwrap_or_default();
        let source_id = slug_segment(&source);
        self.st
            .source_titles
            .entry(source_id.clone())
            .or_default()
            .insert(source);
        let (fields, tags) = match extracted {
            Some(e) => (e.fields.clone(), e.tags.clone()),
            None => (TypedFields::None, Vec::new()),
        };
        RuleEntry {
            id: id.clone(),
            kind,
            title,
            source_ids: vec![source_id],
            section,
            parent,
            body,
            fields,
            tags,
            links: std::mem::take(&mut self.st.cur_links).into_iter().collect(),
            provenance: vec![Provenance {
                dataset: self.plan.dataset.clone(),
                record,
                path,
            }],
            license_refs: vec![LICENSE_OGL.to_string()],
        }
    }

    // ---- typed records ------------------------------------------------------------------

    fn typed_entry(&mut self, t: &'p TypedInput) -> RuleEntry {
        let url = t.node.url.clone().unwrap_or_default();
        self.begin(&t.id, &url, &t.rel);
        let node = &t.node;
        let mut skip_section = None;
        let (mut rows, consumed): (Vec<StatRow>, &[&str]) = match t.kind {
            EntryKind::Spell => self.spell_rows(node, &t.extracted),
            EntryKind::Creature => self.creature_rows(node),
            EntryKind::Item => self.item_rows(node),
            EntryKind::Feat => {
                let (rows, skip) = self.feat_rows(node, &t.extracted);
                skip_section = skip;
                (rows, &["feat_types"])
            }
            EntryKind::Class | EntryKind::PrestigeClass => self.class_rows(node, &t.extracted),
            EntryKind::Skill => self.skill_rows(node),
            EntryKind::Affliction => self.affliction_rows(node),
            EntryKind::Trap => self.trap_rows(node),
            EntryKind::Race | EntryKind::Rule => (Vec::new(), &[]),
        };
        rows.extend(self.extra_rows(&node.rest, consumed));
        let mut body = Vec::new();
        if !rows.is_empty() {
            body.push(Block::StatBlock { title: None, rows });
        }
        self.description_block(node, &mut body);
        self.body_blocks(node, &mut body);
        for (i, section) in node.sections.iter().enumerate() {
            if Some(i) != skip_section {
                self.render_node(section, 0, None, &mut body);
            }
        }
        let section = breadcrumb(&url);
        self.make_entry(
            node,
            Draft {
                id: &t.id,
                kind: t.kind,
                title: t.title.clone(),
                record: url,
                path: t.rel.clone(),
                parent: None,
                section,
                body,
                extracted: Some(&t.extracted),
            },
        )
    }

    fn spell_rows(&mut self, n: &Node, ex: &Extracted) -> (Vec<StatRow>, &'static [&'static str]) {
        const CONSUMED: &[&str] = &[
            "school",
            "subschool",
            "subschool_text",
            "descriptor",
            "descriptor_text",
            "levels",
            "level_text",
            "casting_time",
            "component_text",
            "components",
            "range",
            "effects",
            "duration",
            "saving_throw",
            "spell_resistance",
            "as_spell_id",
        ];
        self.ignore(n, &["as_spell_id"]);
        let mut rows = Vec::new();
        let mut school = n.str_field("school").unwrap_or_default().to_string();
        let sub = n
            .str_field("subschool_text")
            .map(str::to_string)
            .or_else(|| string_values(n.rest.get("subschool")));
        if let Some(sub) = sub.filter(|s| !s.is_empty()) {
            school.push_str(&format!(" ({sub})"));
        }
        let desc = n
            .str_field("descriptor_text")
            .map(str::to_string)
            .or_else(|| string_values(n.rest.get("descriptor")));
        if let Some(desc) = desc.filter(|s| !s.is_empty()) {
            school.push_str(&format!(" [{desc}]"));
        }
        self.push_row(&mut rows, "School", Some(&school));
        let level = n.str_field("level_text").map(str::to_string).or_else(|| {
            let levels = n.rest.get("levels")?.as_array()?;
            let parts: Vec<String> = levels
                .iter()
                .filter_map(|l| {
                    Some(format!(
                        "{} {}",
                        l.get("class")?.as_str()?,
                        l.get("level")?.as_u64()?
                    ))
                })
                .collect();
            Some(parts.join(", "))
        });
        self.push_row(&mut rows, "Level", level.as_deref());
        self.push_row(&mut rows, "Casting Time", n.str_field("casting_time"));
        let components = n
            .str_field("component_text")
            .map(str::to_string)
            .or_else(|| {
                let TypedFields::Spell(f) = &ex.fields else {
                    return None;
                };
                f.components.clone()
            });
        self.push_row(&mut rows, "Components", components.as_deref());
        self.push_row(&mut rows, "Range", n.str_field("range"));
        if let Some(Value::Array(effects)) = n.rest.get("effects") {
            for e in effects {
                let label = e.get("name").and_then(Value::as_str).unwrap_or("Effect");
                self.push_row(&mut rows, label, e.get("text").and_then(Value::as_str));
            }
        }
        self.push_row(&mut rows, "Duration", n.str_field("duration"));
        self.push_row(&mut rows, "Saving Throw", n.str_field("saving_throw"));
        self.push_row(
            &mut rows,
            "Spell Resistance",
            n.str_field("spell_resistance"),
        );
        (rows, CONSUMED)
    }

    fn creature_rows(&mut self, n: &Node) -> (Vec<StatRow>, &'static [&'static str]) {
        const CONSUMED: &[&str] = &[
            "cr",
            "xp",
            "alignment",
            "size",
            "creature_type",
            "creature_subtype",
            "super_race",
            "sex",
            "level",
            "init",
            "senses",
            "aura",
            "ac",
            "hp",
            "hit_dice",
            "fortitude",
            "reflex",
            "will",
            "defensive_abilities",
            "dr",
            "immune",
            "resist",
            "sr",
            "weaknesses",
            "speed",
            "melee",
            "ranged",
            "space",
            "reach",
            "special_attacks",
            "spells",
            "strength",
            "dexterity",
            "constitution",
            "intelligence",
            "wisdom",
            "charisma",
            "base_attack",
            "cmb",
            "cmd",
            "feats",
            "skills",
            "racial_modifiers",
            "languages",
            "special_qualities",
            "gear",
            "combat_gear",
            "other_gear",
            "environment",
            "organization",
            "treasure",
            "boon",
            "mr",
            "natural_armor",
            "breath_weapon",
            "abbrev",
            "subtype",
        ];
        let mut rows = Vec::new();
        let s = |k: &str| n.str_field(k);
        for (key, label) in [
            ("cr", "CR"),
            ("xp", "XP"),
            ("alignment", "Alignment"),
            ("size", "Size"),
        ] {
            self.push_row(&mut rows, label, s(key));
        }
        let kind = match (s("creature_type"), s("creature_subtype")) {
            (Some(t), Some(sub)) => Some(format!("{t} ({sub})")),
            (Some(t), None) => Some(t.to_string()),
            (None, Some(sub)) => Some(format!("({sub})")),
            (None, None) => None,
        };
        self.push_row(&mut rows, "Type", kind.as_deref());
        for (key, label) in [
            ("super_race", "Race"),
            ("sex", "Sex"),
            ("level", "Class Levels"),
            ("init", "Init"),
            ("senses", "Senses"),
            ("aura", "Aura"),
            ("ac", "AC"),
            ("hp", "hp"),
            ("hit_dice", "Hit Dice"),
            ("fortitude", "Fort"),
            ("reflex", "Ref"),
            ("will", "Will"),
            ("defensive_abilities", "Defensive Abilities"),
            ("dr", "DR"),
            ("immune", "Immune"),
            ("resist", "Resist"),
            ("sr", "SR"),
            ("weaknesses", "Weaknesses"),
            ("speed", "Speed"),
            ("melee", "Melee"),
            ("ranged", "Ranged"),
            ("space", "Space"),
            ("reach", "Reach"),
            ("special_attacks", "Special Attacks"),
        ] {
            self.push_row(&mut rows, label, s(key));
        }
        if let Some(Value::Array(spells)) = n.rest.get("spells") {
            for entry in spells {
                if let Value::Object(map) = entry {
                    for (key, value) in map {
                        if let Some(raw) = value.as_str() {
                            let mut chars = key.chars();
                            let label = chars.next().map_or_else(String::new, |f| {
                                f.to_uppercase().collect::<String>() + chars.as_str()
                            });
                            self.push_row(&mut rows, &label, Some(raw));
                        }
                    }
                }
            }
        }
        let keys = [
            ("strength", "Str"),
            ("dexterity", "Dex"),
            ("constitution", "Con"),
            ("intelligence", "Int"),
            ("wisdom", "Wis"),
            ("charisma", "Cha"),
        ];
        if keys.iter().any(|(k, _)| n.rest.contains_key(*k)) {
            let line = keys
                .iter()
                .map(|(k, l)| format!("{l} {}", s(k).unwrap_or("-")))
                .collect::<Vec<_>>()
                .join(", ");
            self.push_row(&mut rows, "Ability Scores", Some(&line));
        }
        for (key, label) in [
            ("base_attack", "Base Atk"),
            ("cmb", "CMB"),
            ("cmd", "CMD"),
            ("feats", "Feats"),
            ("skills", "Skills"),
            ("racial_modifiers", "Racial Modifiers"),
            ("languages", "Languages"),
            ("special_qualities", "SQ"),
            ("gear", "Gear"),
            ("combat_gear", "Combat Gear"),
            ("other_gear", "Other Gear"),
            ("environment", "Environment"),
            ("organization", "Organization"),
            ("treasure", "Treasure"),
            ("boon", "Boon"),
            ("mr", "MR"),
            ("natural_armor", "Natural Armor"),
            ("breath_weapon", "Breath Weapon"),
            ("abbrev", "Abbreviation"),
        ] {
            self.push_row(&mut rows, label, s(key));
        }
        (rows, CONSUMED)
    }

    fn item_rows(&mut self, n: &Node) -> (Vec<StatRow>, &'static [&'static str]) {
        const CONSUMED: &[&str] = &["price", "weight", "slot", "aura", "cl", "misc"];
        let mut rows = Vec::new();
        if let Some(sub) = n.subtype.as_deref() {
            self.push_row(&mut rows, "Type", Some(sub));
        }
        let slot = n.str_field("slot").map(str::to_lowercase);
        self.push_row(&mut rows, "Slot", slot.as_deref());
        self.push_row(&mut rows, "Aura", n.str_field("aura"));
        self.push_row(&mut rows, "CL", n.str_field("cl"));
        self.push_row(&mut rows, "Price", n.str_field("price"));
        self.push_row(&mut rows, "Weight", n.str_field("weight"));
        for (sub, label, raw) in fields::misc_pairs(n).unwrap_or_default() {
            self.push_row(&mut rows, &fields::misc_label(&sub, &label), Some(&raw));
        }
        (rows, CONSUMED)
    }

    fn feat_rows(&mut self, n: &Node, ex: &Extracted) -> (Vec<StatRow>, Option<usize>) {
        let mut rows = Vec::new();
        let types = match &ex.fields {
            TypedFields::Feat(f) => Some(f.feat_types.join(", ")),
            _ => None,
        }
        .filter(|s| !s.is_empty());
        self.push_row(&mut rows, "Type", types.as_deref());
        // A simple prerequisites section moves into the header; anything richer stays a section.
        let skip = n.sections.iter().position(|s| {
            s.name.as_deref().is_some_and(|nm| {
                matches!(
                    nm.trim().to_lowercase().as_str(),
                    "prerequisites" | "prerequisite"
                )
            }) && s.sections.is_empty()
                && s.url.is_none()
                && s.body.as_deref().is_some_and(|b| {
                    let blocks = html::to_blocks(b, &mut html::NullEnv);
                    blocks.len() == 1 && matches!(blocks[0], Block::Paragraph { .. })
                })
        });
        if let Some(i) = skip {
            let body = n.sections[i].body.clone().unwrap_or_default();
            self.push_row(&mut rows, "Prerequisites", Some(&body));
        }
        (rows, skip)
    }

    fn class_rows(&mut self, n: &Node, ex: &Extracted) -> (Vec<StatRow>, &'static [&'static str]) {
        const CONSUMED: &[&str] = &["hit_die", "alignment", "image", "alt"];
        self.ignore(n, &["image", "alt"]);
        let mut rows = Vec::new();
        let class_type = match &ex.fields {
            TypedFields::Class(f) => f.class_type.clone(),
            _ => None,
        };
        self.push_row(&mut rows, "Class Type", class_type.as_deref());
        self.push_row(&mut rows, "Hit Die", n.str_field("hit_die"));
        self.push_row(&mut rows, "Alignment", n.str_field("alignment"));
        (rows, CONSUMED)
    }

    fn skill_rows(&mut self, n: &Node) -> (Vec<StatRow>, &'static [&'static str]) {
        const CONSUMED: &[&str] = &["attribute", "trained_only", "armor_check_penalty"];
        let mut rows = Vec::new();
        self.push_row(&mut rows, "Key Ability", n.str_field("attribute"));
        self.push_row(
            &mut rows,
            "Trained Only",
            yes_no(n.rest.get("trained_only")),
        );
        self.push_row(
            &mut rows,
            "Armor Check Penalty",
            yes_no(n.rest.get("armor_check_penalty")),
        );
        (rows, CONSUMED)
    }

    fn affliction_rows(&mut self, n: &Node) -> (Vec<StatRow>, &'static [&'static str]) {
        const CONSUMED: &[&str] = &[
            "contracted",
            "save",
            "onset",
            "frequency",
            "effect",
            "initial_effect",
            "secondary_effect",
            "cure",
        ];
        let mut rows = Vec::new();
        self.push_row(&mut rows, "Type", n.subtype.as_deref());
        for (key, label) in [
            ("contracted", "Contracted"),
            ("save", "Save"),
            ("onset", "Onset"),
            ("frequency", "Frequency"),
            ("effect", "Effect"),
            ("initial_effect", "Initial Effect"),
            ("secondary_effect", "Secondary Effect"),
            ("cure", "Cure"),
        ] {
            self.push_row(&mut rows, label, n.str_field(key));
        }
        (rows, CONSUMED)
    }

    fn trap_rows(&mut self, n: &Node) -> (Vec<StatRow>, &'static [&'static str]) {
        const CONSUMED: &[&str] = &[
            "trap_type",
            "cr",
            "perception",
            "disable_device",
            "trigger",
            "reset",
            "duration",
            "effect",
        ];
        let mut rows = Vec::new();
        for (key, label) in [
            ("trap_type", "Type"),
            ("cr", "CR"),
            ("perception", "Perception"),
            ("disable_device", "Disable Device"),
            ("trigger", "Trigger"),
            ("reset", "Reset"),
            ("duration", "Duration"),
            ("effect", "Effect"),
        ] {
            self.push_row(&mut rows, label, n.str_field(key));
        }
        (rows, CONSUMED)
    }

    // ---- rules trees --------------------------------------------------------------------

    fn rules_entries(&mut self, r: &'p RulesPlan, entries: &mut Vec<RuleEntry>) {
        let url = r.node.url.clone().unwrap_or_default();
        let mut children_entries = Vec::new();

        // Children first so that the parent's links and anchors stay independent.
        for child in &r.children {
            if let ChildPlan::Entry { id, idx } = child {
                let node = &r.node.sections[*idx];
                let curl = node
                    .url
                    .clone()
                    .unwrap_or_else(|| format!("{url}#{}", node.title()));
                self.begin(id, &curl, &r.rel);
                if let Some(u) = &node.url {
                    self.register(u, None);
                }
                let mut body = Vec::new();
                self.description_block(node, &mut body);
                let rows = self.extra_rows(&node.rest, &[]);
                if !rows.is_empty() {
                    body.push(Block::StatBlock { title: None, rows });
                }
                self.body_blocks(node, &mut body);
                self.render_children(&node.sections, 0, None, &mut body);
                let section = if node.url.is_some() {
                    breadcrumb(&curl)
                } else {
                    let mut s = breadcrumb(&url);
                    s.push(r.title.clone());
                    s
                };
                let mut node_for_source = node.clone();
                if node_for_source.source.is_none() {
                    node_for_source.source = r.node.source.clone();
                }
                let entry = self.make_entry(
                    &node_for_source,
                    Draft {
                        id,
                        kind: EntryKind::Rule,
                        title: html::plain_text(node.title()),
                        record: curl.clone(),
                        path: r.rel.clone(),
                        parent: Some(r.id.clone()),
                        section,
                        body,
                        extracted: None,
                    },
                );
                children_entries.push(entry);
            }
        }

        self.begin(&r.id, &url, &r.rel);
        let mut body = Vec::new();
        self.description_block(&r.node, &mut body);
        let rows = self.extra_rows(&r.node.rest, &[]);
        if !rows.is_empty() {
            body.push(Block::StatBlock { title: None, rows });
        }
        self.body_blocks(&r.node, &mut body);
        for child in &r.children {
            match child {
                ChildPlan::Typed { url, idx } => {
                    let t = &self.plan.typed[self.plan.typed_by_url[url]];
                    let copy = &r.node.sections[*idx];
                    self.note_nested_copy(copy, t);
                    self.st.cur_links.insert(EntryReference {
                        target: t.id.clone(),
                        anchor: None,
                    });
                    body.push(Block::EntryLink {
                        target: t.id.clone(),
                        title: t.title.clone(),
                    });
                }
                ChildPlan::Entry { id, idx } | ChildPlan::Dup { id, idx } => {
                    self.st.cur_links.insert(EntryReference {
                        target: id.clone(),
                        anchor: None,
                    });
                    body.push(Block::EntryLink {
                        target: id.clone(),
                        title: html::plain_text(r.node.sections[*idx].title()),
                    });
                }
                ChildPlan::Inline { idx } => {
                    let node = &r.node.sections[*idx];
                    self.render_node(node, 0, None, &mut body);
                }
            }
        }
        let parent = self.make_entry(
            &r.node,
            Draft {
                id: &r.id,
                kind: EntryKind::Rule,
                title: r.title.clone(),
                record: url.clone(),
                path: r.rel.clone(),
                parent: None,
                section: breadcrumb(&url),
                body,
                extracted: None,
            },
        );
        entries.push(parent);
        entries.extend(children_entries);
    }
}

fn string_values(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Array(items) => {
            let parts: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}
