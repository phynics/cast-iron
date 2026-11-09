//! Pure lookups over the schema-3 index files: relations, spell lists, and book order.
//!
//! Nothing here touches the browser. `Library` loads the files lazily and wraps each in one of
//! these types, parsed once. Every function that walks a graph has a cycle guard and a size cap,
//! because the data is untrusted input.

use cast_iron_schema::{
    Block, BookIndex, BookNode, BookTree, EntryId, Inline, RelationIndex, RelationKind,
    SpellListClass, SpellListDetail, SpellLists,
};
use std::collections::{BTreeMap, HashMap, HashSet};

// ---- relations --------------------------------------------------------------------------

/// One entry's edges: kind, target, and an optional label.
type Edges = Vec<(RelationKind, EntryId, Option<String>)>;

/// `relations.json` indexed both ways. The index holds each edge once, from the entry that
/// states it; this adds the inverse lookup that the entry screens need.
#[derive(Debug, Clone, Default)]
pub struct Relations {
    outgoing: HashMap<EntryId, Edges>,
    incoming: HashMap<EntryId, Edges>,
    /// Edge count per kind slug, as the compiler wrote it.
    pub counts: BTreeMap<String, u32>,
}

/// A feat in a chain, with the feats one step further along the walk: the feats it requires
/// when walking up, the feats that require it when walking down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainNode {
    pub id: EntryId,
    pub next: Vec<ChainNode>,
    /// True when the walk stopped here because the id is already on the path above it.
    pub cycle: bool,
    /// True when more feats lie beyond this node but the depth or size cap ended the walk.
    pub cut: bool,
}

/// A feat that other feats require and that requires none itself: where a chain starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainHead {
    pub id: EntryId,
    /// Feats that name this one directly.
    pub direct: usize,
    /// Every feat that depends on this one, directly or not.
    pub total: usize,
}

/// Depth and size caps for chain walks. The longest chain in the library has 5 feats.
const MAX_DEPTH: usize = 8;
const MAX_NODES: usize = 200;

impl Relations {
    pub fn new(index: RelationIndex) -> Self {
        let mut outgoing: HashMap<EntryId, Vec<(RelationKind, EntryId, Option<String>)>> =
            HashMap::new();
        let mut incoming: HashMap<EntryId, Vec<(RelationKind, EntryId, Option<String>)>> =
            HashMap::new();
        for edge in index.edges {
            let cast_iron_schema::RelationEdge(source, kind, target, label) = edge;
            outgoing
                .entry(source.clone())
                .or_default()
                .push((kind, target.clone(), label.clone()));
            incoming
                .entry(target)
                .or_default()
                .push((kind, source, label));
        }
        Self {
            outgoing,
            incoming,
            counts: index.counts,
        }
    }

    /// A discovery-only view containing edges whose two endpoints are eligible. Filtering
    /// the original edges (rather than reconnecting reachable nodes) preserves provenance.
    pub fn filtered(&self, eligible: impl Fn(&EntryId) -> bool) -> Self {
        let mut outgoing = HashMap::new();
        let mut incoming = HashMap::new();
        let mut counts = BTreeMap::<String, u32>::new();
        for (source, edges) in &self.outgoing {
            if !eligible(source) {
                continue;
            }
            for (kind, target, label) in edges {
                if !eligible(target) {
                    continue;
                }
                outgoing
                    .entry(source.clone())
                    .or_insert_with(Vec::new)
                    .push((*kind, target.clone(), label.clone()));
                incoming
                    .entry(target.clone())
                    .or_insert_with(Vec::new)
                    .push((*kind, source.clone(), label.clone()));
                *counts.entry(kind.slug().to_owned()).or_default() += 1;
            }
        }
        Self {
            outgoing,
            incoming,
            counts,
        }
    }

    /// Entries that point at `target` with this relation: the racial traits of a race, the
    /// archetypes of a class, the feats that require a feat. In index order (sorted by id).
    pub fn sources_of(&self, target: &EntryId, kind: RelationKind) -> Vec<&EntryId> {
        self.incoming
            .get(target)
            .into_iter()
            .flatten()
            .filter(|(k, _, _)| *k == kind)
            .map(|(_, id, _)| id)
            .collect()
    }

    /// Entries that `source` points at with this relation, in index order.
    pub fn targets_of(&self, source: &EntryId, kind: RelationKind) -> Vec<&EntryId> {
        self.outgoing
            .get(source)
            .into_iter()
            .flatten()
            .filter(|(k, _, _)| *k == kind)
            .map(|(_, id, _)| id)
            .collect()
    }

    /// Feats that name `feat` in their prerequisites.
    pub fn leads_to(&self, feat: &EntryId) -> Vec<&EntryId> {
        self.sources_of(feat, RelationKind::Prerequisite)
    }

    /// The add-ons that attach to `target` with `label` (`mythic`, `stamina`, `changes`), in
    /// index order. The ids are the add-ons' own entries, not their bases.
    pub fn addons_of(&self, target: &EntryId, label: &str) -> Vec<&EntryId> {
        self.incoming
            .get(target)
            .into_iter()
            .flatten()
            .filter(|(kind, _, found)| {
                *kind == RelationKind::Addon && found.as_deref() == Some(label)
            })
            .map(|(_, id, _)| id)
            .collect()
    }

    /// The label of the add-on relation that `id` holds, when it is an add-on: `mythic`,
    /// `stamina` or `changes`. An add-on attaches to one base, so the first edge answers.
    pub fn addon_label(&self, id: &EntryId) -> Option<&str> {
        self.outgoing
            .get(id)
            .into_iter()
            .flatten()
            .find(|(kind, _, _)| *kind == RelationKind::Addon)
            .and_then(|(_, _, label)| label.as_deref())
    }

    /// Every revision edge as `(revision, label, base)`, sorted by revision id and then base,
    /// so a caller that lists revisions gets one stable order without reading shards.
    pub fn revision_edges(&self) -> Vec<(&EntryId, Option<&str>, &EntryId)> {
        let mut edges: Vec<(&EntryId, Option<&str>, &EntryId)> = self
            .outgoing
            .iter()
            .flat_map(|(source, edges)| {
                edges.iter().filter_map(move |(kind, target, label)| {
                    (*kind == RelationKind::Revision).then_some((source, label.as_deref(), target))
                })
            })
            .collect();
        edges.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.2.cmp(b.2)));
        edges
    }

    /// The base that `revision` revises, with the edge's label (`None` means revised).
    /// An entry holds at most one revision edge, so the first match answers.
    pub fn revision_base(&self, revision: &EntryId) -> Option<(&EntryId, Option<&str>)> {
        self.outgoing
            .get(revision)
            .into_iter()
            .flatten()
            .find_map(|(kind, target, label)| {
                (*kind == RelationKind::Revision).then_some((target, label.as_deref()))
            })
    }

    /// A view for an entry page where `base` holds `revision`'s prerequisite edges in place of
    /// its own. A revision in force changes what the feat requires, so its chain must be walked
    /// from the revised text. Only the outgoing prerequisite edges move; every other edge keeps
    /// its source and target.
    pub fn with_prerequisite_override(&self, base: &EntryId, revision: &EntryId) -> Self {
        let mut view = self.clone();
        if let Some(edges) = view.outgoing.get_mut(base) {
            edges.retain(|(kind, _, _)| *kind != RelationKind::Prerequisite);
        }
        let revised: Vec<(RelationKind, EntryId, Option<String>)> = self
            .outgoing
            .get(revision)
            .into_iter()
            .flatten()
            .filter(|(kind, _, _)| *kind == RelationKind::Prerequisite)
            .cloned()
            .collect();
        if !revised.is_empty() {
            view.outgoing
                .entry(base.clone())
                .or_default()
                .extend(revised);
        }
        view
    }

    /// The outgoing prerequisite edges of one entry, with their either-or labels. A revision
    /// carries the prerequisites of its revised text this way.
    pub fn prerequisite_edges(&self, source: &EntryId) -> Vec<(EntryId, Option<String>)> {
        self.outgoing
            .get(source)
            .into_iter()
            .flatten()
            .filter(|(kind, _, _)| *kind == RelationKind::Prerequisite)
            .map(|(_, target, label)| (target.clone(), label.clone()))
            .collect()
    }

    /// A graph with the rows of an active revision selection applied: `hidden` entries leave
    /// with every edge that touches them, `redirect` sends an edge into a merged base to the
    /// feat that absorbed it, and `prerequisites` replaces one entry's outgoing prerequisite
    /// edges. Everything else keeps its source, target and label.
    pub fn with_effective_revisions(
        &self,
        hidden: impl Fn(&EntryId) -> bool,
        redirect: &BTreeMap<EntryId, EntryId>,
        prerequisites: &BTreeMap<EntryId, Vec<(EntryId, Option<String>)>>,
    ) -> Self {
        fn push(
            outgoing: &mut HashMap<EntryId, Edges>,
            incoming: &mut HashMap<EntryId, Edges>,
            counts: &mut BTreeMap<String, u32>,
            source: &EntryId,
            kind: RelationKind,
            target: &EntryId,
            label: &Option<String>,
        ) {
            outgoing
                .entry(source.clone())
                .or_default()
                .push((kind, target.clone(), label.clone()));
            incoming
                .entry(target.clone())
                .or_default()
                .push((kind, source.clone(), label.clone()));
            *counts.entry(kind.slug().to_owned()).or_default() += 1;
        }
        let resolve = |target: &EntryId| -> Option<EntryId> {
            let target = if hidden(target) {
                redirect.get(target)?.clone()
            } else {
                target.clone()
            };
            (!hidden(&target)).then_some(target)
        };
        let mut outgoing = HashMap::new();
        let mut incoming = HashMap::new();
        let mut counts = BTreeMap::new();
        for (source, edges) in &self.outgoing {
            if hidden(source) {
                continue;
            }
            let overridden = prerequisites.get(source).is_some();
            for (kind, target, label) in edges {
                if *kind == RelationKind::Prerequisite && overridden {
                    continue;
                }
                if let Some(target) = resolve(target) {
                    push(
                        &mut outgoing,
                        &mut incoming,
                        &mut counts,
                        source,
                        *kind,
                        &target,
                        label,
                    );
                }
            }
        }
        for (source, edges) in prerequisites {
            if hidden(source) {
                continue;
            }
            for (target, label) in edges {
                if let Some(target) = resolve(target) {
                    push(
                        &mut outgoing,
                        &mut incoming,
                        &mut counts,
                        source,
                        RelationKind::Prerequisite,
                        &target,
                        label,
                    );
                }
            }
        }
        Self {
            outgoing,
            incoming,
            counts,
        }
    }

    /// Every add-on entry and the label of the edge it holds, for rows that name their kind
    /// without opening the add-on's shard. The graph's edges decide the label; an id never does.
    pub fn addon_labels(&self) -> HashMap<EntryId, String> {
        self.outgoing
            .iter()
            .filter_map(|(id, edges)| {
                let label = edges.iter().find_map(|(kind, _, label)| {
                    (*kind == RelationKind::Addon)
                        .then(|| label.clone())
                        .flatten()
                })?;
                Some((id.clone(), label))
            })
            .collect()
    }

    /// The prerequisite tree above `feat`. The root is `feat` itself. A feat reached twice by
    /// different routes appears twice, which shows the diamond; a feat that is its own ancestor
    /// stops with `cycle` set. The walk ends at [`MAX_DEPTH`] or after [`MAX_NODES`] nodes, and
    /// the nodes where it ended say so with `cut`.
    pub fn prerequisite_chain(&self, feat: &EntryId) -> ChainNode {
        self.tree(feat, true)
    }

    /// The tree of feats that build on `feat`, walked the other way, with the same guards.
    pub fn dependents_tree(&self, feat: &EntryId) -> ChainNode {
        self.tree(feat, false)
    }

    fn tree(&self, feat: &EntryId, upward: bool) -> ChainNode {
        let mut budget = MAX_NODES;
        let mut path: Vec<&EntryId> = Vec::new();
        self.walk(feat, upward, &mut path, &mut budget)
    }

    fn walk<'a>(
        &'a self,
        id: &'a EntryId,
        upward: bool,
        path: &mut Vec<&'a EntryId>,
        budget: &mut usize,
    ) -> ChainNode {
        *budget = budget.saturating_sub(1);
        if path.contains(&id) {
            return ChainNode {
                id: id.clone(),
                next: Vec::new(),
                cycle: true,
                cut: false,
            };
        }
        path.push(id);
        let following = if upward {
            self.targets_of(id, RelationKind::Prerequisite)
        } else {
            self.leads_to(id)
        };
        let mut next = Vec::new();
        let mut cut = false;
        if path.len() > MAX_DEPTH {
            cut = !following.is_empty();
        } else {
            for follow in &following {
                if *budget == 0 {
                    cut = true;
                    break;
                }
                next.push(self.walk(follow, upward, path, budget));
            }
        }
        path.pop();
        ChainNode {
            id: id.clone(),
            next,
            cycle: false,
            cut,
        }
    }

    /// Every feat above `feat`, once each, nearest first. Used to count a chain without the
    /// repeats that [`Relations::prerequisite_chain`] shows.
    pub fn ancestors(&self, feat: &EntryId) -> Vec<EntryId> {
        self.reach(feat, true)
    }

    /// Every feat that depends on `feat`, directly or not, once each, nearest first.
    pub fn descendants(&self, feat: &EntryId) -> Vec<EntryId> {
        self.reach(feat, false)
    }

    fn reach(&self, start: &EntryId, upward: bool) -> Vec<EntryId> {
        let mut seen: HashSet<&EntryId> = HashSet::new();
        seen.insert(start);
        let mut order: Vec<EntryId> = Vec::new();
        let mut level: Vec<&EntryId> = vec![start];
        for _ in 0..MAX_DEPTH * 2 {
            let mut next: Vec<&EntryId> = Vec::new();
            for id in &level {
                let found = if upward {
                    self.targets_of(id, RelationKind::Prerequisite)
                } else {
                    self.leads_to(id)
                };
                for found in found {
                    if seen.insert(found) {
                        next.push(found);
                        order.push(found.clone());
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            level = next;
        }
        order
    }

    /// Feats that lead to more than `more_than` other feats down their chain, with that number.
    /// Every feat that depends on one, directly or not, counts once. A list page marks these
    /// rows, so the work is done once for the whole graph and not per row.
    pub fn long_chains(&self, more_than: usize) -> HashMap<EntryId, usize> {
        self.incoming
            .iter()
            .filter(|(_, edges)| {
                edges
                    .iter()
                    .any(|(k, _, _)| *k == RelationKind::Prerequisite)
            })
            .filter_map(|(id, _)| {
                let total = self.descendants(id).len();
                (total > more_than).then(|| (id.clone(), total))
            })
            .collect()
    }

    /// Where chains start: feats with dependents and no prerequisite feat. Most depended-on
    /// first, then by id.
    pub fn chain_heads(&self) -> Vec<ChainHead> {
        let mut heads: Vec<ChainHead> = self
            .incoming
            .iter()
            .filter_map(|(id, edges)| {
                let direct = edges
                    .iter()
                    .filter(|(k, _, _)| *k == RelationKind::Prerequisite)
                    .count();
                let has_prerequisite = self.outgoing.get(id).is_some_and(|out| {
                    out.iter().any(|(k, _, _)| *k == RelationKind::Prerequisite)
                });
                (direct > 0 && !has_prerequisite).then(|| ChainHead {
                    id: id.clone(),
                    direct,
                    total: self.descendants(id).len(),
                })
            })
            .collect();
        heads.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.id.cmp(&b.id)));
        heads
    }
}

/// Clone a spell-list index while removing disabled spells and class-entry links. The shared
/// spell array stays canonical so each list's indices remain valid; level indices are pruned.
pub fn filtered_spell_lists(lists: &SpellLists, eligible: impl Fn(&EntryId) -> bool) -> SpellLists {
    let mut result = lists.clone();
    for list in &mut result.classes {
        if list.class.as_ref().is_some_and(|id| !eligible(id)) {
            list.class = None;
        }
        for level in &mut list.levels {
            level.spells.retain(|&i| {
                result
                    .spells
                    .get(i as usize)
                    .is_some_and(|spell| eligible(&spell.0))
            });
        }
        list.levels.retain(|level| !level.spells.is_empty());
    }
    result.classes.retain(|list| !list.levels.is_empty());
    result
}

/// True for an alternate racial trait. The compiler marks these with the url segment
/// `Alternate Racial Traits`, and the id keeps that segment as a slug. The race page needs the
/// split without opening the shard of every trait.
pub fn is_alternate_racial_trait(id: &EntryId) -> bool {
    id.as_str().contains("/alternate-racial-traits/")
}

// ---- spell lists ------------------------------------------------------------------------

/// The list for a class name, ignoring case.
pub fn find_list<'a>(lists: &'a SpellLists, name: &str) -> Option<&'a SpellListClass> {
    lists
        .classes
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(name.trim()))
}

/// The list of a class entry.
pub fn list_of_class_entry<'a>(
    lists: &'a SpellLists,
    class: &EntryId,
) -> Option<&'a SpellListClass> {
    lists
        .classes
        .iter()
        .find(|c| c.class.as_ref() == Some(class))
}

/// `(level, spell count)` of a list, ascending.
pub fn level_counts(list: &SpellListClass) -> Vec<(u8, usize)> {
    list.levels
        .iter()
        .map(|l| (l.level, l.spells.len()))
        .collect()
}

/// Spells in a list, once each, whatever the level.
pub fn list_total(list: &SpellListClass) -> usize {
    list.levels.iter().map(|l| l.spells.len()).sum()
}

/// One spell of a list with what its row shows. `detail` is `None` in a library built before
/// rows carried details.
#[derive(Debug, Clone, Copy)]
pub struct ListedSpell<'a> {
    pub id: &'a EntryId,
    pub title: &'a str,
    pub detail: Option<&'a SpellListDetail>,
}

impl<'a> ListedSpell<'a> {
    fn at(lists: &'a SpellLists, position: u32) -> Option<Self> {
        let spell = lists.spells.get(position as usize)?;
        Some(Self {
            id: &spell.0,
            title: &spell.1,
            detail: lists.details.get(position as usize),
        })
    }

    fn school(&self) -> &'a str {
        self.detail.map_or("", |d| d.school.as_str())
    }
}

/// The spells of one list, grouped by level, narrowed to one level, by a text filter and by
/// school. The text is a case-insensitive substring of the title; blank keeps everything. No
/// school keeps everything; several keep a spell of any of them. Spells keep the list's order
/// (by title). Levels with no match are left out.
pub fn filter_list<'a>(
    lists: &'a SpellLists,
    list: &'a SpellListClass,
    level: Option<u8>,
    filter: &str,
    schools: &[String],
) -> Vec<(u8, Vec<ListedSpell<'a>>)> {
    let needle = filter.trim().to_lowercase();
    list.levels
        .iter()
        .filter(|l| level.is_none_or(|wanted| wanted == l.level))
        .filter_map(|l| {
            let spells: Vec<ListedSpell> = l
                .spells
                .iter()
                .filter_map(|&pos| ListedSpell::at(lists, pos))
                .filter(|spell| needle.is_empty() || spell.title.to_lowercase().contains(&needle))
                .filter(|spell| schools.is_empty() || schools.iter().any(|s| s == spell.school()))
                .collect();
            (!spells.is_empty()).then_some((l.level, spells))
        })
        .collect()
}

/// The schools of a list's spells with how many spells each holds, by name. `level` narrows
/// the count to one spell level, so the numbers match the rows on that page, and a school with
/// no spell there is absent. Empty when the library carries no details, so the reader then
/// offers no school filter.
pub fn list_schools(
    lists: &SpellLists,
    list: &SpellListClass,
    level: Option<u8>,
) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for l in list
        .levels
        .iter()
        .filter(|l| level.is_none_or(|wanted| wanted == l.level))
    {
        for &pos in &l.spells {
            if let Some(spell) = ListedSpell::at(lists, pos) {
                let school = spell.school();
                if !school.is_empty() {
                    *counts.entry(school).or_default() += 1;
                }
            }
        }
    }
    counts
        .into_iter()
        .map(|(s, n)| (s.to_string(), n))
        .collect()
}

// ---- prestige class requirements ---------------------------------------------------------

/// The blocks under the `Requirements` heading of a class, as stated, up to the next heading of
/// the same or a higher level. Headings inside the section, including those nested in lists and
/// quotes, become bold lines, so that the copy has no anchors and no second `id` on the page.
/// Empty when the entry has no such heading.
pub fn requirements_blocks(body: &[Block]) -> Vec<Block> {
    let heading_text = |content: &[Inline]| -> String {
        crate::views::rich::plain_text(content)
            .trim()
            .to_lowercase()
    };
    let Some((start, level)) = body.iter().enumerate().find_map(|(i, block)| match block {
        Block::Heading { level, content, .. }
            if heading_text(content).starts_with("requirements") =>
        {
            Some((i, *level))
        }
        _ => None,
    }) else {
        return Vec::new();
    };
    body[start + 1..]
        .iter()
        .take_while(|block| !matches!(block, Block::Heading { level: l, .. } if *l <= level))
        .map(demote_headings)
        .collect()
}

/// A copy of `block` in which every heading, at any depth, is a bold paragraph.
pub(crate) fn demote_headings(block: &Block) -> Block {
    match block {
        Block::Heading { content, .. } => Block::Paragraph {
            style: cast_iron_schema::ParagraphStyle::Normal,
            content: vec![Inline::Strong { c: content.clone() }],
        },
        Block::List { ordered, items } => Block::List {
            ordered: *ordered,
            items: items
                .iter()
                .map(|item| item.iter().map(demote_headings).collect())
                .collect(),
        },
        Block::Quote { content } => Block::Quote {
            content: content.iter().map(demote_headings).collect(),
        },
        other => other.clone(),
    }
}

// ---- book order -------------------------------------------------------------------------

/// One entry of a book in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookStop {
    pub entry: EntryId,
    pub title: String,
    /// 0-based path of the node in its book's chapter tree.
    pub path: Vec<usize>,
}

/// An entry's place in a book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookPlace {
    pub book: usize,
    /// Position in [`BookLookup::stops`] of that book.
    pub stop: usize,
}

/// A step of the trail above a node: a chapter or a section heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrailStep {
    pub title: String,
    pub path: Vec<usize>,
}

/// `books.json` with the reading order flattened once. Every entry holds one place in one book
/// (the compiler guarantees this), so a map from entry to place answers "where is this?".
#[derive(Debug, Clone, Default)]
pub struct BookLookup {
    index: BookIndex,
    stops: Vec<Vec<BookStop>>,
    places: HashMap<EntryId, BookPlace>,
}

impl BookLookup {
    pub fn new(index: BookIndex) -> Self {
        let mut stops: Vec<Vec<BookStop>> = Vec::with_capacity(index.books.len());
        let mut places = HashMap::new();
        for (book_i, book) in index.books.iter().enumerate() {
            let mut flat = Vec::new();
            let mut path = Vec::new();
            for (i, chapter) in book.chapters.iter().enumerate() {
                path.push(i);
                flatten(chapter, &mut path, &mut flat);
                path.pop();
            }
            for (stop, item) in flat.iter().enumerate() {
                // A second place for the same entry is ignored; the first one in order wins.
                places
                    .entry(item.entry.clone())
                    .or_insert(BookPlace { book: book_i, stop });
            }
            stops.push(flat);
        }
        Self {
            index,
            stops,
            places,
        }
    }

    pub fn books(&self) -> &[BookTree] {
        &self.index.books
    }

    /// Canonical source ID for an entry's curated book placement, when it has one.
    pub fn source_for_entry(&self, id: &EntryId) -> Option<&str> {
        let place = self.place(id)?;
        self.index
            .books
            .get(place.book)
            .map(|book| book.source.as_str())
    }

    pub fn book(&self, source: &str) -> Option<&BookTree> {
        self.index.books.iter().find(|b| b.source == source)
    }

    pub fn book_at(&self, index: usize) -> Option<&BookTree> {
        self.index.books.get(index)
    }

    /// The reading order of one book: entries only, in chapter order.
    pub fn stops(&self, book: usize) -> &[BookStop] {
        self.stops.get(book).map_or(&[], Vec::as_slice)
    }

    pub fn place(&self, id: &EntryId) -> Option<&BookPlace> {
        self.places.get(id)
    }

    /// The entries before and after `id` in its book.
    pub fn neighbors(&self, id: &EntryId) -> (Option<&BookStop>, Option<&BookStop>) {
        let Some(place) = self.place(id) else {
            return (None, None);
        };
        let stops = self.stops(place.book);
        let before = place.stop.checked_sub(1).and_then(|i| stops.get(i));
        (before, stops.get(place.stop + 1))
    }

    /// The chapters and headings above an entry, outermost first. The entry's own node is not
    /// part of the trail.
    pub fn trail(&self, id: &EntryId) -> Vec<TrailStep> {
        let Some(place) = self.place(id) else {
            return Vec::new();
        };
        let (Some(book), Some(stop)) = (
            self.index.books.get(place.book),
            self.stops(place.book).get(place.stop),
        ) else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        for depth in 1..stop.path.len() {
            let path = stop.path[..depth].to_vec();
            if let Some(node) = node_at(book, &path) {
                steps.push(TrailStep {
                    title: node.title.clone(),
                    path,
                });
            }
        }
        steps
    }

    /// Where a contents node leads: its own entry, or for a heading that holds nested entries,
    /// the nearest entry above it with the heading's anchor.
    pub fn target_of(&self, book: &BookTree, path: &[usize]) -> Option<(EntryId, Option<String>)> {
        let node = node_at(book, path)?;
        if let Some(entry) = &node.entry {
            return Some((entry.clone(), node.anchor.clone()));
        }
        let anchor = node.anchor.clone()?;
        (1..path.len()).rev().find_map(|depth| {
            node_at(book, &path[..depth])
                .and_then(|n| n.entry.clone())
                .map(|entry| (entry, Some(anchor.clone())))
        })
    }
}

pub use cast_iron_schema::books::{ChapterId, chapter_base, chapter_ids, node_at_path};

/// The node at a path, or `None` when the path leaves the tree.
pub fn node_at<'a>(book: &'a BookTree, path: &[usize]) -> Option<&'a BookNode> {
    let (first, rest) = path.split_first()?;
    let mut node = book.chapters.get(*first)?;
    for i in rest {
        node = node.children.get(*i)?;
    }
    Some(node)
}

fn flatten(node: &BookNode, path: &mut Vec<usize>, out: &mut Vec<BookStop>) {
    if let Some(entry) = &node.entry {
        out.push(BookStop {
            entry: entry.clone(),
            title: node.title.clone(),
            path: path.clone(),
        });
    }
    for (i, child) in node.children.iter().enumerate() {
        path.push(i);
        flatten(child, path, out);
        path.pop();
    }
}

/// Entries under a node, itself included: the number a chapter row shows.
pub fn entries_under(node: &BookNode) -> usize {
    usize::from(node.entry.is_some()) + node.children.iter().map(entries_under).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{RelationEdge, SpellListLevel, SpellListSpell};

    fn id(s: &str) -> EntryId {
        EntryId::parse(s).unwrap()
    }

    fn relations(edges: &[(&str, RelationKind, &str)]) -> Relations {
        Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: edges
                .iter()
                .map(|(s, k, t)| RelationEdge::plain(id(s), *k, id(t)))
                .collect(),
        })
    }

    use RelationKind::{Addon, Archetype, Part, Prerequisite};

    #[test]
    fn a_labelled_edge_is_parsed_and_still_answers_kind_lookups() {
        let r = Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: vec![RelationEdge(
                id("c/a"),
                RelationKind::Addon,
                id("c/b"),
                Some("mythic".into()),
            )],
        });
        assert_eq!(r.targets_of(&id("c/a"), RelationKind::Addon), [&id("c/b")]);
        assert_eq!(r.sources_of(&id("c/b"), RelationKind::Addon), [&id("c/a")]);
    }

    #[test]
    fn add_ons_are_found_by_label_and_never_count_as_chain_followers() {
        let edge = |source: &str, kind, target: &str, label: Option<&str>| {
            RelationEdge(id(source), kind, id(target), label.map(String::from))
        };
        let r = Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: vec![
                edge("c/base/mythic", Addon, "c/base", Some("mythic")),
                edge("c/base/stamina", Addon, "c/base", Some("stamina")),
                edge("c/other/change", Addon, "c/other", Some("changes")),
                edge("c/follower", Prerequisite, "c/base", None),
            ],
        });
        // The label selects the edge; source and target keep their directions.
        assert_eq!(r.addons_of(&id("c/base"), "mythic"), [&id("c/base/mythic")]);
        assert_eq!(
            r.addons_of(&id("c/base"), "stamina"),
            [&id("c/base/stamina")]
        );
        assert!(r.addons_of(&id("c/base"), "changes").is_empty());
        assert_eq!(r.addon_label(&id("c/base/mythic")), Some("mythic"));
        assert_eq!(r.addon_label(&id("c/base")), None);
        let labels = r.addon_labels();
        assert_eq!(
            labels.get(&id("c/other/change")).map(String::as_str),
            Some("changes")
        );
        // An add-on is not a follower: only prerequisite edges make a chain.
        assert_eq!(r.leads_to(&id("c/base")), [&id("c/follower")]);
        assert_eq!(r.descendants(&id("c/base")), [id("c/follower")]);
        for addon in ["c/base/mythic", "c/base/stamina"] {
            assert!(!r.long_chains(0).contains_key(&id(addon)));
            assert!(!r.chain_heads().iter().any(|head| head.id == id(addon)));
        }
    }

    #[test]
    fn a_revision_edge_names_its_base_and_label() {
        let edge = |source: &str, target: &str, label: Option<&str>| {
            RelationEdge(
                id(source),
                RelationKind::Revision,
                id(target),
                label.map(String::from),
            )
        };
        let r = Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: vec![
                edge("c/base/eitr", "c/base", None),
                edge("c/base/removed-eitr", "c/base", Some("removed")),
                RelationEdge(
                    id("c/revision"),
                    RelationKind::Prerequisite,
                    id("c/other"),
                    None,
                ),
            ],
        });
        assert_eq!(
            r.revision_base(&id("c/base/eitr")),
            Some((&id("c/base"), None))
        );
        assert_eq!(
            r.revision_base(&id("c/base/removed-eitr")),
            Some((&id("c/base"), Some("removed")))
        );
        assert_eq!(r.revision_base(&id("c/revision")), None);
        // Only revision edges are listed; a prerequisite is not a revision.
        let edges = r.revision_edges();
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].0, &id("c/base/eitr"));
        assert_eq!(edges[1].2, &id("c/base"));
    }

    #[test]
    fn a_prerequisite_override_keeps_the_rest_of_the_graph() {
        let r = Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: vec![
                RelationEdge(id("c/base"), RelationKind::Prerequisite, id("c/old"), None),
                RelationEdge(
                    id("c/base/eitr"),
                    RelationKind::Prerequisite,
                    id("c/new"),
                    None,
                ),
                RelationEdge(
                    id("c/base/eitr"),
                    RelationKind::Revision,
                    id("c/base"),
                    None,
                ),
                RelationEdge(id("c/base"), RelationKind::Part, id("c/part"), None),
            ],
        });
        let view = r.with_prerequisite_override(&id("c/base"), &id("c/base/eitr"));
        assert_eq!(
            view.targets_of(&id("c/base"), RelationKind::Prerequisite),
            [&id("c/new")]
        );
        // The part edge stays; the revision edge is untouched.
        assert_eq!(
            view.targets_of(&id("c/base"), RelationKind::Part),
            [&id("c/part")]
        );
        assert_eq!(
            view.revision_base(&id("c/base/eitr")),
            Some((&id("c/base"), None))
        );
        // The original graph is unchanged.
        assert_eq!(
            r.targets_of(&id("c/base"), RelationKind::Prerequisite),
            [&id("c/old")]
        );
    }

    #[test]
    fn inverse_lookups_find_children_of_a_race_a_class_and_a_feat() {
        let r = relations(&[
            ("c/dwarves/a", Part, "c/dwarves"),
            ("c/dwarves/b", Part, "c/dwarves"),
            ("c/elves/a", Part, "c/elves"),
            ("c/breaker", Archetype, "c/barbarian"),
            ("c/improved-trip", Prerequisite, "c/combat-expertise"),
            ("c/improved-trip", Prerequisite, "c/dodge"),
        ]);
        assert_eq!(
            r.sources_of(&id("c/dwarves"), Part),
            [&id("c/dwarves/a"), &id("c/dwarves/b")]
        );
        assert_eq!(
            r.sources_of(&id("c/barbarian"), Archetype),
            [&id("c/breaker")]
        );
        // The kind separates relations that point at the same entry.
        assert!(r.sources_of(&id("c/barbarian"), Part).is_empty());
        assert_eq!(
            r.leads_to(&id("c/combat-expertise")),
            [&id("c/improved-trip")]
        );
        assert_eq!(
            r.targets_of(&id("c/improved-trip"), Prerequisite),
            [&id("c/combat-expertise"), &id("c/dodge")]
        );
        assert!(r.leads_to(&id("c/nothing")).is_empty());
    }

    #[test]
    fn scoped_relations_drop_both_sides_and_recount_without_reconnecting() {
        let all = relations(&[("c/a", Prerequisite, "c/b"), ("c/b", Prerequisite, "c/c")]);
        let scoped = all.filtered(|id| id.as_str() != "c/b");
        assert!(scoped.targets_of(&id("c/a"), Prerequisite).is_empty());
        assert!(scoped.leads_to(&id("c/c")).is_empty());
        assert!(scoped.counts.is_empty());
    }

    #[test]
    fn a_scoped_graph_keeps_an_add_on_only_while_both_ends_are_eligible() {
        let all = Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: vec![
                RelationEdge(
                    id("c/base/mythic"),
                    Addon,
                    id("c/base"),
                    Some("mythic".into()),
                ),
                RelationEdge(
                    id("c/base/stamina"),
                    Addon,
                    id("c/base"),
                    Some("stamina".into()),
                ),
            ],
        });
        // The add-on's own book carries the row: the raw graph keeps its label, and the base's
        // page filters only the add-on's eligibility.
        assert!(all.addon_label(&id("c/base/mythic")).is_some());
        assert_eq!(
            all.addons_of(&id("c/base"), "mythic"),
            [&id("c/base/mythic")]
        );
        // A graph projected both ways drops the edge once either end is inactive.
        let base_off = all.filtered(|id| id.as_str() != "c/base");
        assert!(base_off.addon_label(&id("c/base/mythic")).is_none());
        assert!(base_off.addons_of(&id("c/base"), "mythic").is_empty());
        // Disabling the add-on's book removes the edge, so the base page shows nothing.
        let holder_off = all.filtered(|id| id.as_str() != "c/base/mythic");
        assert_eq!(
            holder_off.addons_of(&id("c/base"), "stamina"),
            [&id("c/base/stamina")]
        );
        assert!(holder_off.addons_of(&id("c/base"), "mythic").is_empty());
    }

    #[test]
    fn scoped_spell_lists_keep_array_positions_and_clear_disabled_class_links() {
        let lists = SpellLists {
            spells: vec![
                SpellListSpell(id("c/spell-a"), "A".into()),
                SpellListSpell(id("c/spell-b"), "B".into()),
            ],
            details: vec![],
            classes: vec![SpellListClass {
                name: "Wizard".into(),
                class: Some(id("c/wizard")),
                levels: vec![SpellListLevel {
                    level: 1,
                    spells: vec![0, 1],
                }],
            }],
        };
        let scoped = filtered_spell_lists(&lists, |id| {
            id.as_str() != "c/spell-a" && id.as_str() != "c/wizard"
        });
        assert_eq!(scoped.classes[0].class, None);
        assert_eq!(scoped.classes[0].levels[0].spells, [1]);
        assert_eq!(scoped.spells.len(), 2); // canonical positions are preserved
    }

    #[test]
    fn the_chain_above_a_feat_lists_every_route_and_ends_at_its_roots() {
        // Greater Trip needs Improved Trip, which needs Combat Expertise; both also need Dodge.
        let r = relations(&[
            ("c/greater-trip", Prerequisite, "c/improved-trip"),
            ("c/improved-trip", Prerequisite, "c/combat-expertise"),
            ("c/improved-trip", Prerequisite, "c/dodge"),
            ("c/combat-expertise", Prerequisite, "c/dodge"),
        ]);
        let chain = r.prerequisite_chain(&id("c/greater-trip"));
        assert_eq!(chain.id, id("c/greater-trip"));
        assert_eq!(chain.next.len(), 1);
        let improved = &chain.next[0];
        let names: Vec<&str> = improved.next.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(names, ["c/combat-expertise", "c/dodge"]);
        // Dodge shows twice because two routes reach it; the flat list counts it once.
        assert_eq!(improved.next[0].next[0].id, id("c/dodge"));
        assert_eq!(
            r.ancestors(&id("c/greater-trip")),
            [
                id("c/improved-trip"),
                id("c/combat-expertise"),
                id("c/dodge")
            ]
        );
        assert_eq!(
            r.descendants(&id("c/dodge")),
            [
                id("c/improved-trip"),
                id("c/combat-expertise"),
                id("c/greater-trip")
            ]
        );
    }

    #[test]
    fn a_prerequisite_cycle_ends_the_walk() {
        let r = relations(&[
            ("c/a", Prerequisite, "c/b"),
            ("c/b", Prerequisite, "c/c"),
            ("c/c", Prerequisite, "c/a"),
            ("c/loop", Prerequisite, "c/loop"),
        ]);
        let chain = r.prerequisite_chain(&id("c/a"));
        let mut node = &chain;
        let mut seen = vec![node.id.as_str()];
        while let Some(next) = node.next.first() {
            node = next;
            seen.push(node.id.as_str());
        }
        assert_eq!(seen, ["c/a", "c/b", "c/c", "c/a"]);
        assert!(node.cycle && node.next.is_empty());
        let own = r.prerequisite_chain(&id("c/loop"));
        assert!(own.next[0].cycle);
        // Reachability terminates too, and never lists the start.
        assert_eq!(r.ancestors(&id("c/a")), [id("c/b"), id("c/c")]);
        assert!(r.descendants(&id("c/loop")).is_empty());
    }

    #[test]
    fn a_wide_diamond_chain_stops_at_the_size_cap() {
        // 12 layers of 2 feats, each needing both feats of the next layer: 2^12 routes.
        let mut edges = Vec::new();
        let name = |layer: usize, k: usize| format!("c/l{layer}-{k}");
        for layer in 0..12 {
            for k in 0..2 {
                for j in 0..2 {
                    edges.push((name(layer, k), Prerequisite, name(layer + 1, j)));
                }
            }
        }
        let r = Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: edges
                .iter()
                .map(|(s, k, t)| RelationEdge::plain(id(s), *k, id(t)))
                .collect(),
        });
        fn count(n: &ChainNode) -> usize {
            1 + n.next.iter().map(count).sum::<usize>()
        }
        let total = count(&r.prerequisite_chain(&id("c/l0-0")));
        assert!(total <= MAX_NODES + 1, "{total}");
    }

    #[test]
    fn chain_heads_are_roots_that_others_depend_on() {
        let r = relations(&[
            ("c/b", Prerequisite, "c/a"),
            ("c/c", Prerequisite, "c/b"),
            ("c/d", Prerequisite, "c/a"),
            ("c/y", Prerequisite, "c/x"),
        ]);
        let heads = r.chain_heads();
        let seen: Vec<(&str, usize, usize)> = heads
            .iter()
            .map(|h| (h.id.as_str(), h.direct, h.total))
            .collect();
        // `b` has dependents but needs `a`, so only `a` and `x` head a chain.
        assert_eq!(seen, [("c/a", 2, 3), ("c/x", 1, 1)]);
    }

    #[test]
    fn a_feat_that_unlocks_more_than_the_limit_is_a_long_chain_and_each_feat_counts_once() {
        // a -> b, c; b -> d, e; c -> e, f (e is reached twice; the count has it once).
        let r = relations(&[
            ("c/b", Prerequisite, "c/a"),
            ("c/c", Prerequisite, "c/a"),
            ("c/d", Prerequisite, "c/b"),
            ("c/e", Prerequisite, "c/b"),
            ("c/e", Prerequisite, "c/c"),
            ("c/f", Prerequisite, "c/c"),
            // Other relations never count toward a chain.
            ("c/x", Part, "c/a"),
            ("c/y", Archetype, "c/a"),
        ]);
        let long = r.long_chains(4);
        assert_eq!(long.len(), 1);
        assert_eq!(long[&id("c/a")], 5);
        // The limit is exclusive: five dependents are not "more than 5".
        assert!(r.long_chains(5).is_empty());
        // `b` and `c` unlock two and two (e once each), below any limit above 1.
        assert_eq!(r.long_chains(1).len(), 3);
        assert!(!r.long_chains(2).contains_key(&id("c/b")));
        // A cycle terminates and a feat never counts itself.
        let cyclic = relations(&[
            ("c/b", Prerequisite, "c/a"),
            ("c/a", Prerequisite, "c/b"),
            ("c/s", Prerequisite, "c/s"),
        ]);
        assert!(cyclic.long_chains(1).is_empty());
        assert_eq!(cyclic.long_chains(0)[&id("c/a")], 1);
        assert!(!cyclic.long_chains(0).contains_key(&id("c/s")));
    }

    #[test]
    fn alternate_traits_are_found_by_their_id() {
        assert!(is_alternate_racial_trait(&id(
            "advanced-race-guide/races/catfolk/alternate-racial-traits/climber"
        )));
        assert!(!is_alternate_racial_trait(&id(
            "core-rulebook/races/humans/human-racial-traits/skilled"
        )));
    }

    fn lists() -> SpellLists {
        let spell = |s: &str, t: &str| SpellListSpell(id(s), t.into());
        SpellLists {
            spells: vec![
                spell("c/a", "Acid Arrow"),
                spell("c/b", "Burning Hands"),
                spell("c/f", "Fireball"),
                spell("c/m", "Magic Missile"),
            ],
            classes: vec![
                SpellListClass {
                    name: "Wizard".into(),
                    class: Some(id("c/classes/wizard")),
                    levels: vec![
                        SpellListLevel {
                            level: 1,
                            spells: vec![1, 3],
                        },
                        SpellListLevel {
                            level: 2,
                            spells: vec![0],
                        },
                        SpellListLevel {
                            level: 3,
                            spells: vec![2],
                        },
                    ],
                },
                SpellListClass {
                    name: "Hunter".into(),
                    class: None,
                    levels: vec![SpellListLevel {
                        level: 1,
                        spells: vec![3],
                    }],
                },
            ],
            details: ["evocation", "evocation", "evocation", "evocation"]
                .iter()
                .enumerate()
                .map(|(i, school)| SpellListDetail {
                    school: if i == 0 { "conjuration" } else { school }.into(),
                    ..Default::default()
                })
                .collect(),
        }
    }

    #[test]
    fn spell_lists_filter_by_level_and_text() {
        let lists = lists();
        let wizard = find_list(&lists, "wizard").unwrap();
        assert_eq!(level_counts(wizard), [(1, 2), (2, 1), (3, 1)]);
        assert_eq!(list_total(wizard), 4);
        let titles = |groups: Vec<(u8, Vec<ListedSpell>)>| -> Vec<(u8, Vec<String>)> {
            groups
                .into_iter()
                .map(|(l, s)| (l, s.into_iter().map(|s| s.title.to_string()).collect()))
                .collect()
        };
        // One level.
        assert_eq!(
            titles(filter_list(&lists, wizard, Some(1), "", &[])),
            [(
                1,
                vec!["Burning Hands".to_string(), "Magic Missile".to_string()]
            )]
        );
        // Text across levels, case-insensitive, with padding.
        assert_eq!(
            titles(filter_list(&lists, wizard, None, "  AR ", &[])),
            [(2, vec!["Acid Arrow".to_string()])]
        );
        assert_eq!(
            titles(filter_list(&lists, wizard, None, "I", &[])),
            [
                (
                    1,
                    vec!["Burning Hands".to_string(), "Magic Missile".to_string()]
                ),
                (2, vec!["Acid Arrow".to_string()]),
                (3, vec!["Fireball".to_string()]),
            ]
        );
        // A level with no match disappears; an unknown level gives nothing.
        assert!(filter_list(&lists, wizard, Some(3), "acid", &[]).is_empty());
        assert!(filter_list(&lists, wizard, Some(9), "", &[]).is_empty());
        // A broken position in the file is skipped, not a panic.
        let mut broken = lists.clone();
        broken.classes[0].levels[0].spells.push(99);
        let wizard = find_list(&broken, "Wizard").unwrap();
        assert_eq!(filter_list(&broken, wizard, Some(1), "", &[]).len(), 1);
    }

    #[test]
    fn spell_lists_filter_by_school_and_count_the_schools() {
        let lists = lists();
        let wizard = find_list(&lists, "wizard").unwrap();
        assert_eq!(
            list_schools(&lists, wizard, None),
            [("conjuration".to_string(), 1), ("evocation".to_string(), 3)]
        );
        // One level counts only its own spells; a school with none there is not offered.
        // Level 1 holds Burning Hands and Magic Missile, both evocation; Acid Arrow
        // (conjuration) is at level 2.
        assert_eq!(
            list_schools(&lists, wizard, Some(1)),
            [("evocation".to_string(), 2)]
        );
        assert_eq!(
            list_schools(&lists, wizard, Some(2)),
            [("conjuration".to_string(), 1)]
        );
        assert!(list_schools(&lists, wizard, Some(9)).is_empty());
        let titles = |schools: &[&str]| -> Vec<String> {
            let schools: Vec<String> = schools.iter().map(|s| s.to_string()).collect();
            filter_list(&lists, wizard, None, "", &schools)
                .into_iter()
                .flat_map(|(_, s)| s.into_iter().map(|s| s.title.to_string()))
                .collect()
        };
        assert_eq!(titles(&["conjuration"]), ["Acid Arrow"]);
        assert_eq!(titles(&["conjuration", "evocation"]).len(), 4);
        assert!(titles(&["necromancy"]).is_empty());
        // A library without details has no schools and a school filter matches nothing.
        let mut bare = lists.clone();
        bare.details.clear();
        let wizard = find_list(&bare, "wizard").unwrap();
        assert!(list_schools(&bare, wizard, None).is_empty());
    }

    #[test]
    fn lists_are_found_by_name_or_class_entry() {
        let lists = lists();
        assert_eq!(find_list(&lists, "HUNTER").unwrap().name, "Hunter");
        assert!(find_list(&lists, "Bard").is_none());
        assert_eq!(
            list_of_class_entry(&lists, &id("c/classes/wizard"))
                .unwrap()
                .name,
            "Wizard"
        );
        // A list without a class entry is reachable by name only.
        assert!(list_of_class_entry(&lists, &id("c/classes/hunter")).is_none());
    }

    fn node(title: &str, entry: Option<&str>, children: Vec<BookNode>) -> BookNode {
        BookNode {
            title: title.into(),
            entry: entry.map(id),
            anchor: None,
            review: None,
            children,
        }
    }

    fn books() -> BookLookup {
        let mut heading = node(
            "Racial Traits",
            None,
            vec![node("Hardy", Some("b/hardy"), vec![])],
        );
        heading.anchor = Some("racial-traits".into());
        BookLookup::new(BookIndex {
            books: vec![
                BookTree {
                    source: "b".into(),
                    title: "Book B".into(),
                    basis: "From memory.".into(),
                    review: vec![],
                    chapters: vec![
                        node(
                            "Races",
                            None,
                            vec![
                                node("Zebra Folk", Some("b/zebra"), vec![]),
                                node("Dwarves", Some("b/dwarves"), vec![heading]),
                                node("Aasimar", Some("b/aasimar"), vec![]),
                            ],
                        ),
                        node(
                            "Combat",
                            Some("b/combat"),
                            vec![node("Actions", Some("b/actions"), vec![])],
                        ),
                    ],
                },
                BookTree {
                    source: "c".into(),
                    title: "Book C".into(),
                    basis: String::new(),
                    review: vec![],
                    chapters: vec![node("Only", Some("c/only"), vec![])],
                },
            ],
        })
    }

    #[test]
    fn reading_order_follows_the_tree_never_the_alphabet() {
        let books = books();
        let order: Vec<&str> = books.stops(0).iter().map(|s| s.entry.as_str()).collect();
        assert_eq!(
            order,
            [
                "b/zebra",
                "b/dwarves",
                "b/hardy",
                "b/aasimar",
                "b/combat",
                "b/actions"
            ]
        );
        let (before, after) = books.neighbors(&id("b/dwarves"));
        assert_eq!(before.unwrap().entry, id("b/zebra"));
        // The next stop is the nested trait, not the next sibling.
        assert_eq!(after.unwrap().entry, id("b/hardy"));
        // The ends have one neighbour only, and neighbours never cross into another book.
        assert!(books.neighbors(&id("b/zebra")).0.is_none());
        assert!(books.neighbors(&id("b/actions")).1.is_none());
        assert_eq!(books.neighbors(&id("c/only")), (None, None));
        assert_eq!(books.neighbors(&id("b/unknown")), (None, None));
    }

    #[test]
    fn chapter_ids_use_the_anchor_or_a_title_slug_and_never_the_position() {
        let books = books();
        let tree = books.book("b").unwrap();
        let paths = [vec![0], vec![1]];
        let ids = chapter_ids(&tree.chapters, paths.iter().map(Vec::as_slice));
        assert_eq!(
            ids,
            [
                ChapterId {
                    id: "races".into(),
                    path: vec![0]
                },
                ChapterId {
                    id: "combat".into(),
                    path: vec![1]
                },
            ]
        );
        // The anchor wins over the title, and a title with no ASCII letters still gets one id.
        let mut anchored = node("Printed Heading", None, vec![]);
        anchored.anchor = Some("printed-anchor".into());
        let empty = node("— : —", None, vec![]);
        let ids = chapter_ids(&[anchored, empty], [vec![0].as_slice(), vec![1].as_slice()]);
        assert_eq!(ids[0].id, "printed-anchor");
        assert_eq!(ids[1].id, "chapter");
    }

    #[test]
    fn a_chapter_id_collision_gets_a_suffix_without_moving_the_first() {
        let tree = [
            node("Magic", None, vec![]),
            node("Magic", None, vec![]),
            node("Magic", None, vec![]),
            node("Magic", None, vec![]),
        ];
        let paths: Vec<Vec<usize>> = (0..4).map(|i| vec![i]).collect();
        let ids = chapter_ids(&tree, paths.iter().map(Vec::as_slice));
        let names: Vec<&str> = ids.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(names, ["magic", "magic-2", "magic-3", "magic-4"]);
        // Moving the rows does not rename them as long as every base stays present.
        let shifted = chapter_ids(&tree, paths.iter().rev().map(Vec::as_slice));
        let names: Vec<&str> = shifted.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(names, ["magic", "magic-2", "magic-3", "magic-4"]);
        assert_eq!(shifted[0].path, vec![3]);
    }

    #[test]
    fn the_trail_names_the_chapters_above_an_entry() {
        let books = books();
        let trail: Vec<(String, Vec<usize>)> = books
            .trail(&id("b/hardy"))
            .into_iter()
            .map(|s| (s.title, s.path))
            .collect();
        assert_eq!(
            trail,
            [
                ("Races".to_string(), vec![0]),
                ("Dwarves".to_string(), vec![0, 1]),
                ("Racial Traits".to_string(), vec![0, 1, 0]),
            ]
        );
        // A chapter opened as an entry has no trail above it.
        assert!(books.trail(&id("b/combat")).is_empty());
        assert!(books.trail(&id("nowhere")).is_empty());
    }

    #[test]
    fn a_heading_node_opens_its_host_entry_at_the_anchor() {
        let books = books();
        let book = books.book("b").unwrap();
        assert_eq!(
            books.target_of(book, &[0, 1, 0]),
            Some((id("b/dwarves"), Some("racial-traits".to_string())))
        );
        assert_eq!(
            books.target_of(book, &[0, 1]),
            Some((id("b/dwarves"), None))
        );
        // A chapter that only groups has no target; a path outside the tree has none either.
        assert_eq!(books.target_of(book, &[0]), None);
        assert_eq!(books.target_of(book, &[9, 9]), None);
        assert_eq!(entries_under(&book.chapters[0]), 4);
    }

    #[test]
    fn requirements_are_copied_without_anchors() {
        let heading = |level, text: &str, anchor: &str| Block::Heading {
            level,
            anchor: Some(anchor.into()),
            content: vec![Inline::text(text)],
        };
        let para = |text: &str| Block::Paragraph {
            style: cast_iron_schema::ParagraphStyle::Normal,
            content: vec![Inline::text(text)],
        };
        let body = vec![
            para("Intro."),
            heading(2, "Requirements", "requirements"),
            para("To qualify:"),
            heading(3, "Base Attack Bonus", "base-attack-bonus"),
            para("+4."),
            heading(2, "Class Skills", "class-skills"),
            para("Not part of it."),
        ];
        let out = requirements_blocks(&body);
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|b| !matches!(b, Block::Heading { .. })));
        assert!(matches!(&out[1], Block::Paragraph { content, .. }
            if matches!(content.as_slice(), [Inline::Strong { .. }])));
        assert!(requirements_blocks(&[para("x")]).is_empty());
    }

    #[test]
    fn requirements_demote_headings_nested_in_lists_and_quotes() {
        let heading = |text: &str| Block::Heading {
            level: 3,
            anchor: Some(text.to_lowercase()),
            content: vec![Inline::text(text)],
        };
        let body = vec![
            heading("Requirements"),
            Block::List {
                ordered: false,
                items: vec![vec![Block::Quote {
                    content: vec![heading("Deep")],
                }]],
            },
            Block::Quote {
                content: vec![heading("Quoted")],
            },
        ];
        let out = requirements_blocks(&body);
        assert_eq!(out.len(), 2);
        fn has_heading(blocks: &[Block]) -> bool {
            blocks.iter().any(|b| match b {
                Block::Heading { .. } => true,
                Block::List { items, .. } => items.iter().any(|i| has_heading(i)),
                Block::Quote { content } => has_heading(content),
                _ => false,
            })
        }
        assert!(!has_heading(&out));
        let Block::Quote { content } = &out[1] else {
            panic!("quote kept");
        };
        assert!(matches!(&content[0], Block::Paragraph { content, .. }
            if matches!(content.as_slice(), [Inline::Strong { .. }])));
    }
}
