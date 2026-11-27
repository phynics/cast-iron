//! Typed renderers for entry content. Every `Block` and `Inline` variant becomes a Leptos
//! node; nothing here builds markup from strings. Pure helpers sit at the top and have native
//! tests.

use super::{footnotes, icons, progression};
use crate::class_sections::Section;
use crate::library::Library;
use crate::outline::{self, Level};
use crate::powers::{self, Placed, Plan};
use crate::route::{self, Route};
use cast_iron_schema::power::{Effect, upgrade_lead};
use cast_iron_schema::{
    Block, EntryId, FeatureFact, Inline, ParagraphStyle, StatRow, Table, TableCell, TableRow,
};
use leptos::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU32, Ordering};

// ---- pure helpers -----------------------------------------------------------------------

pub use cast_iron_schema::headings::{
    RELATED_ANCHOR, RESERVED_ANCHOR_KEYS, heading_keys, headings_in_order, keyed_headings,
    plain_text, slugify, split_heading_tag,
};

/// The DOM id for a heading key. The prefix keeps content ids away from app ids.
pub fn heading_dom_id(key: &str) -> String {
    format!("h-{key}")
}

/// The `scope` of a header cell. `section` is `"head"`, `"body"`, or `"foot"`.
pub fn th_scope(section: &str, cell: &TableCell, index_in_row: usize) -> &'static str {
    if section == "body" && index_in_row == 0 {
        if cell.rowspan > 1 { "rowgroup" } else { "row" }
    } else if cell.colspan > 1 {
        "colgroup"
    } else {
        "col"
    }
}

fn span_attr(value: u16) -> Option<String> {
    (value > 1).then(|| value.to_string())
}

/// Counts the grid cells a table occupies, as `(cells, with_colspan_or_rowspan)`.
/// Used by tests to check the typed table against upstream counts.
pub fn table_stats(table: &Table) -> (usize, usize) {
    let rows = table.head.iter().chain(&table.body).chain(&table.foot);
    let cells: Vec<&TableCell> = rows.flat_map(|r| r.cells.iter()).collect();
    let spanning = cells
        .iter()
        .filter(|c| c.colspan > 1 || c.rowspan > 1)
        .count();
    (cells.len(), spanning)
}

// ---- inline -----------------------------------------------------------------------------

pub fn inlines(items: &[Inline]) -> Vec<AnyView> {
    items.iter().map(inline).collect()
}

/// A field label such as `Benefit:` reads as the typed field name `benefit`.
fn field_label(items: &[Inline]) -> String {
    plain_text(items)
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_lowercase()
}

/// Inlines with the quiet `changed` label after a stat label that a revision changed. The
/// printed feat body puts its fields in one paragraph as bold labels, so this is where a
/// revised page attaches the marker.
fn inlines_with_changed(items: &[Inline], changed: &BTreeSet<String>) -> Vec<AnyView> {
    items
        .iter()
        .map(|item| {
            let node = inline(item);
            let marked = matches!(item, Inline::Strong { c } if changed.contains(&field_label(c)));
            if marked {
                view! {
                    {node}
                    <span class="tag changed-tag">"changed"</span>
                }
                .into_any()
            } else {
                node
            }
        })
        .collect()
}

fn inline(item: &Inline) -> AnyView {
    match item {
        Inline::Text { v } => v.clone().into_any(),
        Inline::Em { c } => view! { <em>{inlines(c)}</em> }.into_any(),
        Inline::Strong { c } => view! { <strong>{inlines(c)}</strong> }.into_any(),
        Inline::Sup { c } => view! { <sup>{inlines(c)}</sup> }.into_any(),
        Inline::Sub { c } => view! { <sub>{inlines(c)}</sub> }.into_any(),
        Inline::Break => view! { <br /> }.into_any(),
        Inline::Link { target, anchor, c } => {
            let href = Route::Entry {
                id: target.clone(),
                anchor: anchor.clone(),
                book: None,
                section: None,
                arch: Default::default(),
            }
            .href();
            view! { <a class="xref" href=href>{inlines(c)}</a> }.into_any()
        }
    }
}

// ---- blocks -----------------------------------------------------------------------------

/// Renders blocks in order. Consecutive entry links share one list.
pub fn blocks(items: &[Block]) -> Vec<AnyView> {
    Renderer::new(items).blocks(items)
}

/// Renders a body. Each heading finds its key by identity in the table that [`keyed_headings`]
/// built from the same slice, so the ids cannot drift from the keys whatever order it renders in.
struct Renderer {
    keys: BTreeMap<*const Block, Option<String>>,
    /// The element each heading renders as, and the level whose look it keeps (#12, L3). A power
    /// name is keyed by the block that it starts at.
    levels: BTreeMap<*const Block, Level>,
    /// Lowercase labels of the stat rows that a revision changed.
    changed: BTreeSet<String>,
    /// The headings where a class page's sections start, by heading key.
    marks: BTreeMap<String, Section>,
    /// The footnotes of an entry page, once its body is split (#507).
    notes: Option<NoteState>,
    /// A view that takes its place right after the class page's Archetypes heading, before the
    /// section's introduction, so the list is the first thing a reader finds there (#170, part D).
    slot: Option<(String, Callback<(), AnyView>)>,
    /// Views that go in before a top-level block of the body, by the block (#739: the Options
    /// section, and the row under a feature that grants a choice from a group).
    before: BTreeMap<*const Block, Vec<Callback<(), AnyView>>>,
    /// Views that go in after the last top-level block.
    at_end: Vec<Callback<(), AnyView>>,
    /// True when the Archetypes heading has just been rendered and the slot is next.
    slot_open: bool,
    /// The entry whose body this is. An `entry-link` row names the listed entry's book, unless
    /// that is this entry's own row book (#461).
    hub: Option<EntryId>,
    /// The listed features (corpus format 4, #540). A heading that `feature_of` finds in them
    /// renders as a feature head.
    features: Vec<FeatureFact>,
}

/// What the renderer knows about the footnotes of the entry it renders: where each note stands,
/// how markers find it, and which markers cited it (for the way back).
struct NoteState {
    entry: EntryId,
    book: Option<String>,
    /// Per note, in order: the final heading key, the printed label, the anchor as written, and
    /// how many main blocks come before it.
    notes: Vec<NoteInfo>,
    /// The ids of the markers that cite each note, in the order they appear.
    cited_by: Vec<Vec<String>>,
    /// The index of the top-level main block being rendered.
    position: usize,
}

impl NoteState {
    /// The note that a superscript cites, if it cites one of this entry, and the new id of the
    /// marker. The citation is recorded here, so the way back lists the markers in the order
    /// this is called: the order the page renders in, whether a marker stands in a paragraph, a
    /// table cell or a stat row (#534).
    fn cite(&mut self, previous: Option<&Inline>, content: &[Inline]) -> Option<(usize, String)> {
        let note = match footnotes::linked_anchor(content, &self.entry) {
            Some(anchor) => self.notes.iter().position(|note| note.anchor == anchor)?,
            None => {
                let label = footnotes::marker_label(previous, content)?;
                footnotes::resolve(
                    self.notes.iter().map(|note| (note.label.as_str(), note.at)),
                    &label,
                    self.position,
                )?
            }
        };
        let id = format!(
            "fnref-{}-{}",
            self.notes[note].key,
            self.cited_by[note].len() + 1
        );
        self.cited_by[note].push(id.clone());
        Some((note, id))
    }
}

struct NoteInfo {
    key: String,
    label: String,
    anchor: String,
    at: usize,
    /// The accessible name of a marker that links here (`Note 1`, `Bonus feat, note B`).
    name: String,
}

impl Renderer {
    fn new(body: &[Block]) -> Self {
        Self::planned(body, &Plan::default())
    }

    /// A renderer for a body whose class-option powers are laid out by `plan`: the power names
    /// are headings too, so they take part in the outline.
    fn planned(body: &[Block], plan: &Plan<'_>) -> Self {
        let keys = cast_iron_sort::map_of(
            keyed_headings(body)
                .into_iter()
                .map(|(block, key)| (std::ptr::from_ref(block), key)),
        );
        Self {
            keys,
            levels: outline_of(body, plan),
            changed: BTreeSet::new(),
            marks: BTreeMap::new(),
            notes: None,
            slot: None,
            before: BTreeMap::new(),
            at_end: Vec::new(),
            slot_open: false,
            hub: None,
            features: Vec::new(),
        }
    }

    /// Views that the page draws among the blocks of `body`: each goes before the block at its
    /// index, and one at `body.len()` goes after the last block.
    fn with_inserts(
        mut self,
        body: &[Block],
        inserts: Vec<(usize, Callback<(), AnyView>)>,
    ) -> Self {
        for (index, view) in inserts {
            match body.get(index) {
                Some(block) => self
                    .before
                    .entry(std::ptr::from_ref(block))
                    .or_default()
                    .push(view),
                None => self.at_end.push(view),
            }
        }
        self
    }

    /// The level of the feature that a heading with this explicit anchor prints: `None` when the
    /// heading is no listed feature, `Some(None)` for a feature with no level. The rule is the
    /// schema's, shared with the search aliases.
    fn feature_of(&self, anchor: Option<&str>) -> Option<Option<u8>> {
        cast_iron_schema::headings::feature_of(anchor, &self.features).map(|fact| fact.level)
    }

    /// How the heading or power that starts at `block` renders. A block the outline does not know
    /// (a heading that the body split off) keeps the level it was stored with.
    fn level_of(&self, block: &Block, stored: u8) -> Level {
        self.levels
            .get(&std::ptr::from_ref(block))
            .copied()
            .unwrap_or_else(|| Level {
                element: outline::printed(stored),
                printed: outline::printed(stored),
            })
    }

    /// The headings of `features` render as feature heads. A fact with no heading in the body is
    /// ignored, so the data may run ahead of the text or the other way round.
    fn with_features(mut self, features: &[FeatureFact]) -> Self {
        self.features = features.to_vec();
        self
    }

    fn with_slot(mut self, slot: Option<(String, Callback<(), AnyView>)>) -> Self {
        self.slot = slot;
        self
    }

    fn with_marks(mut self, marks: Vec<(String, Section)>) -> Self {
        self.marks = cast_iron_sort::map_of(marks);
        self
    }

    fn with_changed(mut self, changed: BTreeSet<String>) -> Self {
        self.changed = changed;
        self
    }
}

impl Renderer {
    fn blocks(&mut self, items: &[Block]) -> Vec<AnyView> {
        self.blocks_of(items.iter(), false, None)
    }

    /// The body of an entry page: the rules text, then its footnotes as one quiet group (#507).
    /// A body without notes renders as [`Self::blocks`] does.
    fn entry_body(
        &mut self,
        body: &[Block],
        entry: EntryId,
        book: Option<String>,
        plan: &Plan<'_>,
    ) -> Vec<AnyView> {
        self.hub = Some(entry.clone());
        let split = footnotes::split(body);
        if split.notes.is_empty() {
            return self.blocks_of(body.iter(), true, Some(plan));
        }
        let notes = split
            .notes
            .iter()
            .map(|note| {
                let key = self
                    .keys
                    .get(&std::ptr::from_ref(note.heading))
                    .cloned()
                    .flatten()
                    .unwrap_or_default();
                let anchor = match note.heading {
                    Block::Heading {
                        anchor: Some(anchor),
                        ..
                    } => anchor.clone(),
                    _ => key.clone(),
                };
                NoteInfo {
                    key,
                    label: note.label.to_string(),
                    anchor,
                    at: note.at,
                    name: footnotes::marker_name(note.label, note.text),
                }
            })
            .collect::<Vec<_>>();
        self.notes = Some(NoteState {
            entry,
            book,
            cited_by: vec![Vec::new(); notes.len()],
            notes,
            position: 0,
        });
        let mut out = self.blocks_of(split.main.iter().copied(), true, Some(plan));
        out.push(self.notes_group(&split.notes));
        out
    }

    /// `top` marks the blocks of the entry body itself, so a marker knows which notes follow it.
    /// A block nested in a list or a quote has the position of the block that holds it. A power
    /// of the `plan` takes the place of its blocks.
    fn blocks_of<'a>(
        &mut self,
        items: impl Iterator<Item = &'a Block>,
        top: bool,
        plan: Option<&Plan<'_>>,
    ) -> Vec<AnyView> {
        let mut out = Vec::new();
        let mut run: Vec<AnyView> = Vec::new();
        let mut covered = 0;
        for (index, item) in items.enumerate() {
            if top && let Some(state) = self.notes.as_mut() {
                state.position = index;
            }
            if covered > 0 {
                covered -= 1;
                continue;
            }
            if top && let Some(views) = self.before.remove(&std::ptr::from_ref(item)) {
                if !run.is_empty() {
                    let list = std::mem::take(&mut run);
                    out.push(view! { <ul class="entry-links">{list}</ul> }.into_any());
                }
                out.extend(views.into_iter().map(|view| view.run(())));
            }
            if let Some(placed) = plan.and_then(|plan| plan.at(item)) {
                if !run.is_empty() {
                    let list = std::mem::take(&mut run);
                    out.push(view! { <ul class="entry-links">{list}</ul> }.into_any());
                }
                covered = placed.len - 1;
                out.push(self.power(placed, self.level_of(item, placed.heading)));
                continue;
            }
            if let Block::EntryLink { target, title } = item {
                run.push(self.entry_link_row(target, title));
                continue;
            }
            if !run.is_empty() {
                let list = std::mem::take(&mut run);
                out.push(view! { <ul class="entry-links">{list}</ul> }.into_any());
            }
            out.push(self.block(item));
            if top && self.slot_open {
                out.extend(self.take_slot());
            }
        }
        if !run.is_empty() {
            out.push(view! { <ul class="entry-links">{run}</ul> }.into_any());
        }
        if top {
            out.extend(
                std::mem::take(&mut self.at_end)
                    .into_iter()
                    .map(|view| view.run(())),
            );
        }
        out
    }

    /// The slot's view, once: the section it ends is over.
    fn take_slot(&mut self) -> Option<AnyView> {
        self.slot_open = false;
        self.slot.take().map(|(_, slot)| slot.run(()))
    }

    fn block(&mut self, item: &Block) -> AnyView {
        match item {
            Block::Heading {
                level,
                content,
                anchor: explicit,
            } => {
                let key = self.keys.get(&std::ptr::from_ref(item)).cloned().flatten();
                debug_assert!(
                    self.keys.contains_key(&std::ptr::from_ref(item)),
                    "a rendered heading is missing from the heading traversal"
                );
                let id = key.as_deref().map(heading_dom_id);
                let section = key.as_ref().and_then(|key| self.marks.get(key)).copied();
                if self
                    .slot
                    .as_ref()
                    .is_some_and(|(wanted, _)| key.as_ref() == Some(wanted))
                {
                    self.slot_open = true;
                }
                let level = self.level_of(item, *level);
                match self.feature_of(explicit.as_deref()) {
                    Some(gained) => feature_head(level, id, section, gained, content),
                    None => heading(level, id, section, None, inlines(content)),
                }
            }
            Block::Paragraph { style, content } => {
                let class = match style {
                    ParagraphStyle::Normal => None,
                    ParagraphStyle::StatLine => Some("stat-line"),
                    ParagraphStyle::Indented => Some("indented"),
                };
                view! { <p class=class>{self.paragraph_content(content)}</p> }.into_any()
            }
            Block::List { ordered, items } => {
                let rows: Vec<AnyView> = items
                    .iter()
                    .map(|cell| view! { <li>{self.blocks(cell)}</li> }.into_any())
                    .collect();
                if *ordered {
                    view! { <ol>{rows}</ol> }.into_any()
                } else {
                    view! { <ul>{rows}</ul> }.into_any()
                }
            }
            Block::Table(table) => match progression::split(table) {
                Some(split) => self.progression_tables(split),
                None => self.rules_table(table, None, None),
            },
            Block::Quote { content } => {
                view! { <blockquote>{self.blocks(content)}</blockquote> }.into_any()
            }
            Block::StatBlock { title, rows } => self.stat_block(title.as_deref(), rows),
            Block::EntryLink { target, title } => {
                let row = self.entry_link_row(target, title);
                view! { <ul class="entry-links">{row}</ul> }.into_any()
            }
        }
    }

    /// One `entry-link` row: the link, then the muted book of the listed entry, then `Inactive`
    /// when every book of that entry is turned off (#461). The book and the label sit after the
    /// link and not inside it, so the link's name stays the entry title. Without a library in
    /// context, or for an entry the catalog does not know, the row is the bare link.
    fn entry_link_row(&self, target: &EntryId, title: &str) -> AnyView {
        let href = Route::entry(target.clone()).href();
        let credit = use_context::<Library>().and_then(|library| {
            let stub = library.stub(target)?;
            let listed = library.row_book(stub);
            let hub = self
                .hub
                .as_ref()
                .and_then(|hub| library.entry_row_book(hub));
            let name = library
                .source(listed)
                .map_or(listed, |source| &source.title);
            Some((
                listed_book_label(listed, name, hub),
                !library.is_entry_active(target),
            ))
        });
        let (book, inactive) = credit.unwrap_or((None, false));
        let book = book.map(|book| {
            view! {
                <span class="entry-link-book" data-testid="entry-link-book">
                    <span aria-hidden="true">"· "</span>{book}
                </span>
            }
        });
        let label = inactive.then(|| view! { <span class="tag">"Inactive"</span> });
        view! {
            <li data-inactive=inactive.then_some("true")>
                <a href=href>{title.to_string()}</a>
                {book}
                {label}
            </li>
        }
        .into_any()
    }
}

/// The book an `entry-link` row names after its link (#461): the listed entry's row book, by its
/// display title. A row never repeats the book of the page it stands on, so a hub that lists
/// entries of its own book shows no label there. `hub_book` is the page's own row book, when
/// known.
fn listed_book_label(listed: &str, title: &str, hub_book: Option<&str>) -> Option<String> {
    (hub_book != Some(listed)).then(|| title.to_string())
}

impl Renderer {
    /// A power of a class option (decision `2026-11-20-class-option-power-layout`): the head and
    /// the meta line, then the rows in the order the corpus wrote them. The renderer never
    /// reorders and never changes the case of the corpus text. A folded power (a one-power page)
    /// has no head and no meta line: the entry head carries the name and the facts.
    fn power(&mut self, placed: &Placed<'_>, level: Level) -> AnyView {
        let power = &placed.power;
        let mut parts: Vec<AnyView> = Vec::new();
        if !placed.folded {
            parts.push(power_head(placed, level));
            let meta = powers::meta_parts(power);
            if !meta.is_empty() {
                parts.push(muted_line("power-meta", &meta));
            }
        }
        if let Some(flavor) = power.flavor {
            parts.push(self.power_line(flavor));
        }
        for row in &power.preface {
            parts.push(self.labeled_line(&row.label, row.content));
        }
        for effect in std::iter::once(&power.effect).chain(power.second_effect.as_ref()) {
            parts.push(self.effect_line(effect));
            for paragraph in &effect.continuation {
                parts.push(self.power_line(paragraph));
            }
        }
        for upgrade in &power.upgrades {
            let text = self.paragraph_content(&powers::trimmed(upgrade.text));
            parts.push(
                view! {
                    <p class="power-upgrade"><strong>{upgrade_lead(upgrade.level)}</strong>" "{text}</p>
                }
                .into_any(),
            );
        }
        if let Some(prereq) = power.prereq {
            parts.push(self.labeled_line("Prereq", prereq));
        }
        if let Some(replaces) = power.replaces {
            parts.push(self.labeled_line("Replaces", replaces));
        }
        for row in &power.other {
            parts.push(self.labeled_line(&row.label, row.content));
            for paragraph in &row.continuation {
                parts.push(self.power_line(paragraph));
            }
        }
        view! { <div class="power">{parts}</div> }.into_any()
    }

    /// A paragraph of the power: the base text of a row, or a continuation.
    fn power_line(&mut self, content: &[Inline]) -> AnyView {
        let content = self.paragraph_content(&powers::trimmed(content));
        view! { <p>{content}</p> }.into_any()
    }

    /// A row that keeps its bold label, as it renders on a page without the layout.
    fn labeled_line(&mut self, label: &str, content: &[Inline]) -> AnyView {
        let content = self.paragraph_content(&powers::trimmed(content));
        view! { <p><strong>{format!("{label}:")}</strong>" "{content}</p> }.into_any()
    }

    /// An effect row. `Passive`, `Ability` and an action label show no label: the meta line says
    /// the action. `Benefit` keeps its label, which nothing else carries.
    fn effect_line(&mut self, effect: &Effect<'_>) -> AnyView {
        let quiet = effect.is_action() || matches!(effect.label.as_str(), "Passive" | "Ability");
        if quiet {
            self.power_line(effect.content)
        } else {
            self.labeled_line(&effect.label, effect.content)
        }
    }
}

/// The head of a power: the name as a heading, a dotted leader, and `Level N` from `Gained`.
fn power_head(placed: &Placed<'_>, level: Level) -> AnyView {
    let name = placed.power.name.clone();
    let heading = heading(level, None, None, Some("power-name"), vec![name.into_any()]);
    view! { <div class="power-head">{heading}{level_tail(placed.power.gained)}</div> }.into_any()
}

/// The head of a class feature that the entry lists (corpus format 4, #540): the printed name as
/// the body's own heading, so its anchor and its place in `On this page` stay, in the look of a
/// class-option power head (DESIGN.md, "Class-option powers"). With a level, a dotted leader and
/// `Level N` follow on the right. Without one there is no leader and no right side.
fn feature_head(
    level: Level,
    id: Option<String>,
    section: Option<Section>,
    gained: Option<u8>,
    content: &[Inline],
) -> AnyView {
    // The printed text is unchanged: the name, one space and the tag read as the heading did.
    let (name, tag) = split_heading_tag(content);
    let content: Vec<AnyView> = match tag {
        Some(tag) => vec![
            view! { <span class="feature-name">{inlines(&name)}</span> }.into_any(),
            " ".into_any(),
            view! { <span class="ability-tag">{tag}</span> }.into_any(),
        ],
        None => inlines(&name),
    };
    let heading = heading(level, id, section, Some("power-name"), content);
    view! { <div class="power-head feature-head">{heading}{level_tail(gained)}</div> }.into_any()
}

/// The leader and `Level N` at the right of a head. Nothing when there is no level.
fn level_tail(level: Option<u8>) -> Option<AnyView> {
    level.map(|level| {
        view! {
            <span class="power-leader" aria-hidden="true"></span>
            <span class="power-level">
                <span class="sr-only">"Gained at "</span>{powers::level_text(level)}
            </span>
        }
        .into_any()
    })
}

/// A muted line of facts joined by `·`. The dots are hidden from assistive technology and a
/// visually hidden comma stands in for them.
pub fn muted_line(class: &'static str, parts: &[String]) -> AnyView {
    muted_parts(
        class,
        parts
            .iter()
            .map(|part| part.clone().into_any())
            .collect::<Vec<_>>(),
    )
}

/// [`muted_line`] for facts that hold links or other markup.
pub fn muted_parts(class: &'static str, parts: Vec<AnyView>) -> AnyView {
    let items = parts
        .into_iter()
        .enumerate()
        .map(|(index, part)| {
            // The dot is decorative; a screen reader gets a comma between the facts.
            let sep = (index > 0).then(|| {
                view! {
                    <span class="meta-sep" aria-hidden="true">"·"</span>
                    <span class="sr-only">", "</span>
                }
            });
            view! { {sep}<span>{part}</span> }
        })
        .collect_view();
    view! { <p class=class>{items}</p> }.into_any()
}

impl Renderer {
    /// Paragraph inlines. On an entry page, a footnote marker that cites a note of this entry
    /// becomes a link to it, and gets an id so the note can link back.
    fn paragraph_content(&mut self, items: &[Inline]) -> Vec<AnyView> {
        self.marked_inlines(items, true)
    }

    /// Inlines where a marker becomes a link to its note, in a table cell or a stat row as well
    /// as in a paragraph (#534). `label_changes` adds the `changed` tag after a stat label that
    /// a revision changed, which only the paragraphs of a feat body use. Every marker records
    /// its citation as it renders, so the back links follow the order the page renders in.
    fn marked_inlines(&mut self, items: &[Inline], label_changes: bool) -> Vec<AnyView> {
        let mut out = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            let previous = index.checked_sub(1).and_then(|before| items.get(before));
            match self.marker(previous, item) {
                Some(view) => out.push(view),
                None if label_changes => out.extend(inlines_with_changed(
                    std::slice::from_ref(item),
                    &self.changed,
                )),
                None => out.push(inline(item)),
            }
        }
        out
    }

    fn marker(&mut self, previous: Option<&Inline>, item: &Inline) -> Option<AnyView> {
        let Inline::Sup { c } = item else {
            return None;
        };
        let state = self.notes.as_mut()?;
        let (note, id) = state.cite(previous, c)?;
        let info = &state.notes[note];
        // Both forms render as one link to the note with one accessible name. The corpus's own
        // link (`^[5](ref/...#footnote-h)^`) keeps its text and loses its generic name.
        let shown = match c.as_slice() {
            [Inline::Link { c: inner, .. }] => inner.as_slice(),
            _ => c.as_slice(),
        };
        let href = Route::Entry {
            id: state.entry.clone(),
            anchor: Some(info.key.clone()),
            book: state.book.clone(),
            section: None,
            arch: Default::default(),
        }
        .href();
        let content = vec![
            view! { <a href=href aria-label=info.name.clone()>{inlines(shown)}</a> }.into_any(),
        ];
        Some(view! { <sup class="fn-ref" id=id>{content}</sup> }.into_any())
    }

    /// The notes of the entry as one `<aside>`, after the rules text. The note keeps the DOM id
    /// that its heading would have had, so a link to `#footnote-1` still lands on it.
    fn notes_group(&mut self, notes: &[footnotes::Note<'_>]) -> AnyView {
        let title_id = unique("notes");
        let rows: Vec<AnyView> = notes
            .iter()
            .enumerate()
            .map(|(index, note)| {
                let state = self.notes.as_ref().expect("notes are set before the group");
                let info = &state.notes[index];
                let id = heading_dom_id(&info.key);
                let text = match note.text {
                    Block::Paragraph { content, .. } => inlines(content),
                    _ => Vec::new(),
                };
                let cited = &state.cited_by[index];
                let back: Vec<AnyView> = cited
                    .iter()
                    .enumerate()
                    .map(|(n, marker)| {
                        let href = Route::Entry {
                            id: state.entry.clone(),
                            anchor: Some(marker.clone()),
                            book: state.book.clone(),
                            section: None,
                            arch: Default::default(),
                        }
                        .href();
                        let name = match cited.len() {
                            1 => format!("Back to note {} in the text", info.label),
                            many => format!(
                                "Back to note {} in the text, mention {} of {many}",
                                info.label,
                                n + 1
                            ),
                        };
                        view! {
                            <a class="note-back" href=href aria-label=name>
                                {icons::note_back()}
                            </a>
                        }
                        .into_any()
                    })
                    .collect();
                let label = info.label.clone();
                view! {
                    <li class="note" id=id>
                        <span class="note-label"><span class="sr-only">"Note "</span>{label}</span>
                        <div class="note-body">
                            <p>{text}{back}</p>
                        </div>
                    </li>
                }
                .into_any()
            })
            .collect();
        view! {
            <aside class="notes" aria-labelledby=title_id.clone()>
                <h2 class="notes-title" id=title_id.clone()>"Notes"</h2>
                <ul class="notes-list">{rows}</ul>
            </aside>
        }
        .into_any()
    }
}

/// A heading where a class page section starts is the target of the switcher's link: it carries
/// `data-class-section` and can take focus (`tabindex="-1"`), and no other heading does.
///
/// The element is the outline's (`level.element`), so an entry never skips a level below the
/// page's `h1`. `data-printed-level` carries the level the corpus stored, which is what the
/// styles key on, so a shifted heading looks as it always did (#12, L3).
fn heading(
    level: Level,
    id: Option<String>,
    section: Option<Section>,
    class: Option<&'static str>,
    content: Vec<AnyView>,
) -> AnyView {
    let slug = section.map(Section::slug);
    let tabindex = section.map(|_| "-1");
    let printed = level.printed.to_string();
    match level.element {
        0..=2 => view! {
            <h2 id=id class=class data-class-section=slug data-printed-level=printed tabindex=tabindex>{content}</h2>
        }
        .into_any(),
        3 => view! {
            <h3 id=id class=class data-class-section=slug data-printed-level=printed tabindex=tabindex>{content}</h3>
        }
        .into_any(),
        4 => view! {
            <h4 id=id class=class data-class-section=slug data-printed-level=printed tabindex=tabindex>{content}</h4>
        }
        .into_any(),
        5 => view! {
            <h5 id=id class=class data-class-section=slug data-printed-level=printed tabindex=tabindex>{content}</h5>
        }
        .into_any(),
        _ => view! {
            <h6 id=id class=class data-class-section=slug data-printed-level=printed tabindex=tabindex>{content}</h6>
        }
        .into_any(),
    }
}

/// The outline of a body: the level of each heading, and of each power name that `plan` lays
/// out, in document order. A footnote heading is no part of it, because the notes render as
/// their own group. A folded power has no head, so it is no part of it either.
fn outline_of(body: &[Block], plan: &Plan<'_>) -> BTreeMap<*const Block, Level> {
    fn walk(blocks: &[Block], top: bool, plan: &Plan<'_>, out: &mut Vec<(*const Block, u8, bool)>) {
        for block in blocks {
            match block {
                Block::Heading { .. } if footnotes::is_note_heading(block) => {}
                Block::Heading { level, .. } => out.push((std::ptr::from_ref(block), *level, true)),
                Block::List { items, .. } => {
                    for item in items {
                        walk(item, false, plan, out);
                    }
                }
                Block::Quote { content } => walk(content, false, plan, out),
                _ => {
                    if top
                        && let Some(placed) = plan.at(block)
                        && !placed.folded
                    {
                        out.push((std::ptr::from_ref(block), placed.heading, false));
                    }
                }
            }
        }
    }
    let mut found = Vec::new();
    walk(body, true, plan, &mut found);
    let stored: Vec<(u8, bool)> = found
        .iter()
        .map(|(_, level, sets)| (*level, *sets))
        .collect();
    cast_iron_sort::map_of(
        found
            .iter()
            .map(|(block, _, _)| *block)
            .zip(outline::levels_among(&stored)),
    )
}

static UNIQUE: AtomicU32 = AtomicU32::new(0);

fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", UNIQUE.fetch_add(1, Ordering::Relaxed))
}

/// Stat blocks and tables (#534). Their cells and values go through the same marker logic as a
/// paragraph, so a footnote marker in one is the same link, and the note links back to it. The
/// table and stat block markup is unchanged.
impl Renderer {
    fn stat_block(&mut self, title: Option<&str>, rows: &[StatRow]) -> AnyView {
        let title_id = title.map(|_| unique("stat"));
        let heading = title.map(|t| {
            let id = title_id.clone();
            view! { <p class="stat-title" id=id>{t.to_string()}</p> }
        });
        let rows: Vec<AnyView> = rows
            .iter()
            .map(|row| {
                let changed = self
                    .changed
                    .contains(&row.label.trim().to_lowercase())
                    .then(|| {
                        view! { <span class="tag changed-tag">"changed"</span> }
                    });
                view! {
                    <div class="stat-row">
                        <dt>{row.label.clone()}{changed}</dt>
                        <dd>{self.marked_inlines(&row.value, false)}</dd>
                    </div>
                }
                .into_any()
            })
            .collect();
        view! {
            <div class="stat-block">
                {heading}
                <dl aria-labelledby=title_id>{rows}</dl>
            </div>
        }
        .into_any()
    }

    /// A class progression: the core columns, then any spell columns in a native disclosure. The
    /// core table pins its Level column and fits a phone; the disclosure keeps the slots one tap
    /// away without widening the page. The core columns render before the spell columns, so a
    /// marker in a spell column counts after the markers of every row of the core columns.
    fn progression_tables(&mut self, split: progression::Progression) -> AnyView {
        let core = self.rules_table(
            &split.core,
            Some("progression-table"),
            Some("Class progression"),
        );
        let spells = split.spells.map(|spells| {
            let (summary, region) = progression::spells_labels(&spells);
            let table = self.rules_table(&spells, Some("progression-table"), Some(region));
            view! {
                <details class="progression-spells">
                    <summary>{summary}</summary>
                    {table}
                </details>
            }
        });
        view! { {core}{spells} }.into_any()
    }

    /// `class` adds a modifier class to the table. `label` names the region of a table without a
    /// caption; the default is `Table`.
    fn rules_table(
        &mut self,
        table: &Table,
        class: Option<&'static str>,
        label: Option<&'static str>,
    ) -> AnyView {
        let caption_id = unique("cap");
        let has_caption = !table.caption.is_empty();
        let caption = has_caption.then(|| {
            view! { <caption id=caption_id.clone()>{inlines(&table.caption)}</caption> }
        });
        let label = (!has_caption).then_some(label.unwrap_or("Table"));
        let labelled_by = has_caption.then(|| caption_id.clone());
        let breakable = class.is_some();
        // The sections render in document order, so the citations record in that order.
        let head = (!table.head.is_empty()).then(|| {
            view! { <thead>{self.table_rows(&table.head, "head", breakable)}</thead> }
        });
        let body = view! { <tbody>{self.table_rows(&table.body, "body", breakable)}</tbody> };
        let foot = (!table.foot.is_empty()).then(|| {
            view! { <tfoot>{self.table_rows(&table.foot, "foot", breakable)}</tfoot> }
        });
        let table_class = match class {
            Some(extra) => format!("rules-table {extra}"),
            None => "rules-table".to_string(),
        };
        view! {
            <div class="table-wrap" role="region" tabindex="0" aria-labelledby=labelled_by aria-label=label>
                <table class=table_class>{caption}{head}{body}{foot}</table>
            </div>
        }
        .into_any()
    }

    /// A cell's content. In a progression table an iterative attack bonus gets a break
    /// opportunity after each slash, so the column can narrow and leave room for Special. Each
    /// part is its own `.atk` span. A bonus of three or more parts is stacked, one per line, on a
    /// phone (`atk-stack`); a two-part bonus such as `+6/+1` stays on one line while it fits.
    fn cell_content(&mut self, content: &[Inline], breakable: bool) -> Vec<AnyView> {
        if breakable
            && let [Inline::Text { v }] = content
            && let Some(parts) = progression::attack_bonus_parts(v)
        {
            let span_class = if parts.len() > 2 {
                "atk atk-stack"
            } else {
                "atk"
            };
            return parts
                .into_iter()
                .map(|part| {
                    view! { <span class=span_class>{part.to_string()}</span><wbr /> }.into_any()
                })
                .collect();
        }
        self.marked_inlines(content, false)
    }

    fn table_rows(
        &mut self,
        rows: &[TableRow],
        section: &'static str,
        breakable: bool,
    ) -> Vec<AnyView> {
        rows.iter()
            .map(|row| {
                let cells: Vec<AnyView> = row
                    .cells
                    .iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let colspan = span_attr(cell.colspan);
                        let rowspan = span_attr(cell.rowspan);
                        if cell.header {
                            let scope = th_scope(section, cell, i);
                            view! {
                                <th scope=scope colspan=colspan rowspan=rowspan>
                                    {self.cell_content(&cell.content, breakable)}
                                </th>
                            }
                            .into_any()
                        } else {
                            view! {
                                <td colspan=colspan rowspan=rowspan>{self.cell_content(&cell.content, breakable)}</td>
                            }
                            .into_any()
                        }
                    })
                    .collect();
                view! { <tr>{cells}</tr> }.into_any()
            })
            .collect()
    }
}

// ---- body ---------------------------------------------------------------------------------

/// The body of an entry. The longest entry in the POC library (about 420 blocks, with a table
/// of 340 cells) renders in under 40 ms, and 60 ms with the CPU throttled 4x, so the whole body
/// renders at once. With an anchor, the view scrolls to it after the first frame.
#[component]
pub fn RichBody(
    body: Vec<Block>,
    anchor: Option<String>,
    /// Lowercase labels of stat rows that a revision changed.
    #[prop(optional)]
    changed: Vec<String>,
    /// The headings where a class page's sections start, by heading key.
    #[prop(optional)]
    marks: Vec<(String, Section)>,
    /// The entry this body belongs to, and the book the reader came from. With an entry, the
    /// body's footnotes become one Notes group after the text (#507), and markers link to them.
    #[prop(optional)]
    entry: Option<EntryId>,
    #[prop(into)] book: Option<String>,
    /// The entry's title, set on a class option only: its powers take the layout of decision
    /// `2026-11-20-class-option-power-layout`. Any other entry renders its body as printed.
    #[prop(optional_no_strip)]
    power_title: Option<String>,
    /// A class page's archetype list and the key of the heading it follows.
    #[prop(optional_no_strip)]
    archetype_list: Option<(String, Callback<(), AnyView>)>,
    /// The class features the entry lists (corpus format 4, #540). Their headings render as
    /// feature heads.
    #[prop(optional)]
    features: Vec<FeatureFact>,
    /// Views that the page draws among the top-level blocks: each goes before the block at its
    /// index, and one at `body.len()` goes at the end (#739).
    #[prop(optional_no_strip)]
    inserts: Vec<(usize, Callback<(), AnyView>)>,
) -> impl IntoView {
    let changed: BTreeSet<_> = cast_iron_sort::set_of(changed);
    let plan = power_title
        .map(|title| powers::plan_for(&title, &body, &features))
        .unwrap_or_default();
    let mut renderer = Renderer::planned(&body, &plan)
        .with_changed(changed)
        .with_marks(marks)
        .with_features(&features)
        .with_inserts(&body, inserts)
        .with_slot(archetype_list);
    let content = match entry {
        Some(entry) => renderer.entry_body(&body, entry, book, &plan),
        None => renderer.blocks(&body),
    };
    if let Some(anchor) = anchor {
        request_animation_frame(move || {
            route::scroll_to_anchor(&anchor);
        });
    }
    view! { <div class="rich">{content}</div> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::EntryId;

    fn text(s: &str) -> Inline {
        Inline::text(s)
    }

    #[test]
    fn an_entry_link_row_names_the_listed_entrys_book() {
        assert_eq!(
            listed_book_label(
                "horror-adventures",
                "Horror Adventures",
                Some("aquatic-adventures")
            ),
            Some("Horror Adventures".to_string())
        );
        assert_eq!(
            listed_book_label("horror-adventures", "Horror Adventures", None),
            Some("Horror Adventures".to_string())
        );
    }

    #[test]
    fn an_entry_link_row_never_repeats_the_book_of_its_page() {
        assert_eq!(
            listed_book_label(
                "horror-adventures",
                "Horror Adventures",
                Some("horror-adventures")
            ),
            None
        );
    }

    #[test]
    fn plain_text_flattens_every_inline() {
        let items = vec![
            text("A "),
            Inline::Em {
                c: vec![text("fire")],
            },
            Inline::Break,
            Inline::Strong {
                c: vec![text("ball")],
            },
            Inline::Sup { c: vec![text("2")] },
            Inline::Sub { c: vec![text("x")] },
            Inline::Link {
                target: EntryId::parse("a/b").unwrap(),
                anchor: None,
                c: vec![text("!")],
            },
        ];
        assert_eq!(plain_text(&items), "A fire ball2x!");
    }

    #[test]
    fn slugify_keeps_words() {
        assert_eq!(slugify("Spell Resistance (SR)"), "spell-resistance-sr");
        assert_eq!(slugify("  --  "), "");
        assert_eq!(slugify("Élan 5"), "lan-5");
    }

    fn h(anchor: Option<&str>, title: &str) -> Block {
        Block::Heading {
            level: 2,
            anchor: anchor.map(String::from),
            content: vec![text(title)],
        }
    }

    #[test]
    fn only_a_listed_feature_heading_is_a_feature_head_and_it_carries_its_level() {
        let body = vec![
            h(Some("class-skills"), "Class Skills"),
            h(Some("shattering-strike"), "Shattering Strike (Ex)"),
            h(Some("weapon-training"), "Weapon Training"),
            h(None, "Piledriver"),
        ];
        let fact = |anchor: &str, level| FeatureFact {
            anchor: anchor.into(),
            level,
        };
        let renderer = Renderer::new(&body).with_features(&[
            fact("shattering-strike", Some(2)),
            fact("weapon-training", None),
            // A fact whose heading is not in the body does nothing.
            fact("missing", Some(9)),
        ]);
        let heads: Vec<Option<Option<u8>>> = keyed_headings(&body)
            .into_iter()
            .map(|(block, _)| match block {
                Block::Heading { anchor, .. } => renderer.feature_of(anchor.as_deref()),
                _ => None,
            })
            .collect();
        // A heading that is no feature renders as before; a level-less feature has a head but
        // no level, so no leader and no right side.
        assert_eq!(heads, [None, Some(Some(2)), Some(None), None]);
        // An entry with no features renders every heading as before.
        let plain = Renderer::new(&body);
        assert!(
            keyed_headings(&body)
                .into_iter()
                .all(|(block, _)| match block {
                    Block::Heading { anchor, .. } => plain.feature_of(anchor.as_deref()).is_none(),
                    _ => true,
                })
        );
    }

    #[test]
    fn heading_dom_ids_are_namespaced_away_from_app_ids() {
        let body = vec![h(Some("main"), "Main")];
        let keys = heading_keys(&body);
        assert_eq!(keys, vec![Some("main".to_string())]);
        assert_eq!(heading_dom_id(keys[0].as_deref().unwrap()), "h-main");
    }

    #[test]
    fn duplicate_heading_text_gets_distinct_keys() {
        let body = vec![h(None, "X"), h(None, "X")];
        assert_eq!(
            heading_keys(&body),
            vec![Some("sec-x".to_string()), Some("sec-x-2".to_string())]
        );
    }

    #[test]
    fn duplicate_suffix_skips_a_literal_key_later_in_the_body() {
        let body = vec![h(None, "X"), h(None, "X"), h(Some("sec-x-2"), "Y")];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("sec-x".to_string()),
                Some("sec-x-3".to_string()),
                Some("sec-x-2".to_string()),
            ]
        );
    }

    #[test]
    fn empty_anchor_falls_back_to_the_slug_and_an_empty_slug_has_no_key() {
        let body = vec![h(Some(""), "Big Table"), h(None, "!!")];
        assert_eq!(
            heading_keys(&body),
            vec![Some("sec-big-table".to_string()), None]
        );
    }

    #[test]
    fn heading_keys_follow_nested_blocks_in_document_order() {
        let body = vec![
            h(None, "X"),
            Block::List {
                ordered: false,
                items: vec![vec![h(None, "X")]],
            },
            Block::Quote {
                content: vec![h(Some("q"), "Q")],
            },
        ];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("sec-x".to_string()),
                Some("sec-x-2".to_string()),
                Some("q".to_string()),
            ]
        );
    }

    #[test]
    fn a_content_heading_never_takes_a_reserved_app_anchor() {
        let body = vec![
            h(Some(RELATED_ANCHOR), "Related"),
            h(Some(RELATED_ANCHOR), "Related again"),
            h(None, "Entry Related"),
        ];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("entry-related-2".to_string()),
                Some("entry-related-3".to_string()),
                Some("sec-entry-related".to_string()),
            ]
        );
        assert!(RESERVED_ANCHOR_KEYS.contains(&RELATED_ANCHOR));
    }

    #[test]
    fn a_reserved_anchor_suffix_skips_a_literal_key_in_the_body() {
        let body = vec![
            h(Some(RELATED_ANCHOR), "A"),
            h(Some("entry-related-2"), "B"),
        ];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("entry-related-3".to_string()),
                Some("entry-related-2".to_string()),
            ]
        );
    }

    #[test]
    fn the_traversal_visits_nested_headings_and_the_renderer_table_matches_it() {
        let body = vec![
            h(None, "A"),
            Block::List {
                ordered: true,
                items: vec![
                    vec![h(None, "B")],
                    vec![Block::Quote {
                        content: vec![h(None, "C")],
                    }],
                ],
            },
            Block::Quote {
                content: vec![h(None, "D")],
            },
            h(None, "A"),
        ];
        let visited = headings_in_order(&body);
        let titles: Vec<String> = visited
            .iter()
            .map(|block| match block {
                Block::Heading { content, .. } => plain_text(content),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(titles, ["A", "B", "C", "D", "A"]);
        // The renderer looks each heading up by identity in this table.
        let renderer = Renderer::new(&body);
        assert_eq!(renderer.keys.len(), 5);
        for (block, key) in keyed_headings(&body) {
            assert_eq!(renderer.keys.get(&std::ptr::from_ref(block)), Some(&key));
        }
        let last = visited[4];
        assert_eq!(
            renderer.keys[&std::ptr::from_ref(last)].as_deref(),
            Some("sec-a-2")
        );
    }

    #[test]
    fn th_scope_follows_position_and_span() {
        let cell = |colspan, rowspan| TableCell {
            header: true,
            colspan,
            rowspan,
            content: vec![],
        };
        assert_eq!(th_scope("head", &cell(1, 1), 0), "col");
        assert_eq!(th_scope("head", &cell(3, 1), 1), "colgroup");
        assert_eq!(th_scope("body", &cell(1, 1), 0), "row");
        assert_eq!(th_scope("body", &cell(1, 2), 0), "rowgroup");
        assert_eq!(th_scope("body", &cell(1, 1), 2), "col");
    }

    #[test]
    fn table_stats_count_spans() {
        let plain = TableCell {
            content: vec![text("a")],
            colspan: 1,
            rowspan: 1,
            header: false,
        };
        let span = TableCell {
            colspan: 2,
            ..plain.clone()
        };
        let table = Table {
            caption: vec![],
            head: vec![TableRow {
                cells: vec![span.clone()],
            }],
            body: vec![TableRow {
                cells: vec![plain.clone(), plain],
            }],
            foot: vec![],
        };
        assert_eq!(table_stats(&table), (3, 1));
    }

    fn note(key: &str, label: &str, at: usize) -> NoteInfo {
        NoteInfo {
            key: key.to_string(),
            label: label.to_string(),
            anchor: format!("footnote-{label}").to_lowercase(),
            at,
            name: format!("Note {label}"),
        }
    }

    fn note_state(notes: Vec<NoteInfo>) -> NoteState {
        NoteState {
            entry: EntryId::parse("ref/spell/nightmare").unwrap(),
            book: None,
            cited_by: vec![Vec::new(); notes.len()],
            notes,
            position: 0,
        }
    }

    fn linked_marker(label: &str, anchor: &str) -> Vec<Inline> {
        vec![Inline::Link {
            target: EntryId::parse("ref/spell/nightmare").unwrap(),
            anchor: Some(anchor.to_string()),
            c: vec![text(label)],
        }]
    }

    #[test]
    fn a_cell_marker_cites_its_note_like_a_paragraph_marker() {
        let mut state = note_state(vec![note("footnote-1", "1", 2)]);
        // A paragraph marker, then a linked marker and a bare marker in table cells.
        let paragraph = state.cite(Some(&text("word")), &[text("1")]);
        let linked = state.cite(None, &linked_marker("1", "footnote-1"));
        let bare = state.cite(Some(&text("Yes")), &[text("1")]);
        assert_eq!(paragraph, Some((0, "fnref-footnote-1-1".to_string())));
        assert_eq!(linked, Some((0, "fnref-footnote-1-2".to_string())));
        assert_eq!(bare, Some((0, "fnref-footnote-1-3".to_string())));
        // The way back lists the three citations in the order they were met.
        assert_eq!(
            state.cited_by[0],
            [
                "fnref-footnote-1-1",
                "fnref-footnote-1-2",
                "fnref-footnote-1-3"
            ]
        );
    }

    #[test]
    fn a_cell_marker_that_cites_no_note_of_the_entry_stays_plain() {
        let mut state = note_state(vec![note("footnote-1", "1", 2)]);
        // An exponent, a marker with no such note, and a link into another entry.
        assert_eq!(state.cite(Some(&text("10")), &[text("2")]), None);
        assert_eq!(state.cite(None, &[text("7")]), None);
        let other = vec![Inline::Link {
            target: EntryId::parse("ref/spell/other").unwrap(),
            anchor: Some("footnote-1".to_string()),
            c: vec![text("1")],
        }];
        assert_eq!(state.cite(None, &other), None);
        assert!(state.cited_by[0].is_empty());
    }

    fn heading_block(level: u8, anchor: &str) -> Block {
        Block::Heading {
            level,
            anchor: Some(anchor.to_string()),
            content: vec![text(anchor)],
        }
    }

    #[test]
    fn the_outline_of_a_body_starts_at_h2_and_leaves_the_notes_out() {
        let body = vec![
            heading_block(3, "defense"),
            heading_block(4, "hp"),
            // A list holds a heading too: it is part of the outline.
            Block::List {
                ordered: false,
                items: vec![vec![heading_block(4, "nested")]],
            },
            heading_block(3, "offense"),
            // The footnote heading is level six with a `footnote-` anchor and no part of it.
            heading_block(6, "footnote-1"),
        ];
        let renderer = Renderer::new(&body);
        let outline = outline_of(&body, &Plan::default());
        let elements = |block: &Block| outline.get(&std::ptr::from_ref(block)).map(|l| l.element);
        assert_eq!(elements(&body[0]), Some(2));
        assert_eq!(elements(&body[1]), Some(3));
        assert_eq!(elements(&body[3]), Some(2));
        assert_eq!(elements(&body[4]), None);
        let Block::List { items, .. } = &body[2] else {
            unreachable!()
        };
        assert_eq!(elements(&items[0][0]), Some(3));
        // The look stays the stored level.
        assert_eq!(renderer.level_of(&body[1], 4).printed, 4);
        assert_eq!(renderer.level_of(&body[1], 4).element, 3);
        // A heading the outline does not know keeps its stored level.
        assert_eq!(renderer.level_of(&body[4], 6).element, 6);
    }

    fn paragraph(items: Vec<Inline>) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: items,
        }
    }

    fn marker_cell(label: &str) -> TableCell {
        TableCell {
            header: false,
            colspan: 1,
            rowspan: 1,
            content: vec![
                text("Yes"),
                Inline::Sup {
                    c: vec![text(label)],
                },
            ],
        }
    }

    fn note_blocks(label: &str, note: &str) -> [Block; 2] {
        [
            Block::Heading {
                level: 6,
                anchor: Some(format!("footnote-{}", label.to_lowercase())),
                content: vec![text(label)],
            },
            paragraph(vec![text(note)]),
        ]
    }

    /// The real entry render path, not a hand-set position: a paragraph and a table cite the
    /// first note, a stat row cites the second note, which has the same label.
    #[test]
    fn markers_in_paragraphs_tables_and_stat_rows_cite_by_the_position_of_their_block() {
        let [first_heading, first_text] = note_blocks("B", "First.");
        let [second_heading, second_text] = note_blocks("B", "Second.");
        let body = vec![
            paragraph(vec![text("Feats"), Inline::Sup { c: vec![text("B")] }]),
            Block::Table(Table {
                caption: Vec::new(),
                head: Vec::new(),
                body: vec![TableRow {
                    cells: vec![marker_cell("B"), marker_cell("B")],
                }],
                foot: Vec::new(),
            }),
            first_heading,
            first_text,
            Block::StatBlock {
                title: None,
                rows: vec![StatRow {
                    label: "Feats".to_string(),
                    value: vec![text("Cleave"), Inline::Sup { c: vec![text("B")] }],
                }],
            },
            second_heading,
            second_text,
        ];
        let entry = EntryId::parse("ref/monster/example").unwrap();
        let mut renderer = Renderer::new(&body);
        let owner = Owner::new();
        owner.with(|| {
            renderer.entry_body(&body, entry, None, &Plan::default());
        });
        let state = renderer.notes.as_ref().expect("the entry has notes");
        assert_eq!(state.notes.len(), 2);
        assert_eq!(state.notes[0].key, "footnote-b");
        assert_eq!(state.notes[1].key, "footnote-b-2");
        // Paragraph, then the two cells of the table, in document order, for the first note;
        // the stat row, which stands after the first note, for the second.
        assert_eq!(
            state.cited_by,
            [
                vec![
                    "fnref-footnote-b-1".to_string(),
                    "fnref-footnote-b-2".to_string(),
                    "fnref-footnote-b-3".to_string(),
                ],
                vec!["fnref-footnote-b-2-1".to_string()],
            ]
        );
    }

    #[test]
    fn bare_cell_markers_pick_the_note_by_the_position_of_the_block() {
        // Two stat blocks with a note `B` each: notes stand after blocks 1 and 3.
        let mut state = note_state(vec![
            note("footnote-b", "B", 1),
            note("footnote-b-2", "B", 3),
        ]);
        state.position = 0;
        assert_eq!(state.cite(None, &[text("B")]).map(|c| c.0), Some(0));
        state.position = 2;
        assert_eq!(state.cite(None, &[text("B")]).map(|c| c.0), Some(1));
        state.position = 4;
        assert_eq!(state.cite(None, &[text("B")]).map(|c| c.0), Some(1));
        assert_eq!(state.cited_by[0].len(), 1);
        assert_eq!(
            state.cited_by[1],
            ["fnref-footnote-b-2-1", "fnref-footnote-b-2-2"]
        );
    }
}
