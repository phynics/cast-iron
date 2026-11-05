//! The book view: each publication's chapters in the original order.
//!
//! Chapters and their order come from the curated `book-order.toml`. Inside a chapter, an
//! entry sits where the rendered text puts it: a rules section under its rules tree, a
//! racial trait under its race, an archetype under the class section that holds it. Records
//! that no tree nests, such as spells and feats, are ordered by the upstream sibling order.
//! Nothing is sorted alphabetically unless the upstream order itself is alphabetical.

use crate::build::NestedLink;
use crate::order::{BookOrderFile, BookSpec, ChapterSpec, MatchSpec, UpstreamOrder};
use crate::slug::slug_segment;
use cast_iron_schema::{
    BookIndex, BookNode, BookOrderCoverage, BookPlacement, BookTree, EntryId, EntryKind, RuleEntry,
    Unplaced,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Report {
    pub index: BookIndex,
    pub coverage: BookOrderCoverage,
    /// Counts from [`build_canonical`]. Empty for the curated PSRD builder.
    pub stats: CanonicalBookStats,
}

/// What the canonical page-ordered pass covered.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct CanonicalBookStats {
    pub books: usize,
    pub entries: usize,
    /// Entries with at least one page number, so they hold a page place in a book.
    pub placed_by_page: usize,
    /// Entries with a publication but no page number in any source line.
    pub unpaged: usize,
    /// Entries whose source attribution names more than one publication.
    pub multiple_books: usize,
    /// Entries that no publication names, so no book can hold them.
    pub no_publication: usize,
    /// Entry-to-book memberships. One entry can belong to more than one book.
    pub placements: usize,
    pub paged_placements: usize,
    pub unpaged_placements: usize,
}

const PAGE_CHAPTER: &str = "Page order";
const UNPAGED_CHAPTER: &str = "Unpaged";

/// The upstream url of an entry, without the fragment that unnamed sections carry.
pub fn home_url(entry: &RuleEntry) -> &str {
    let record = entry
        .provenance
        .first()
        .map(|p| p.record.as_str())
        .unwrap_or_default();
    record.split('#').next().unwrap_or(record)
}

fn matches(m: &MatchSpec, entry: &RuleEntry, home: &str) -> bool {
    let under = home == m.url
        || home
            .strip_prefix(m.url.as_str())
            .is_some_and(|rest| rest.starts_with('/'));
    if !under {
        return false;
    }
    if !m.kinds.is_empty() && !m.kinds.iter().any(|k| k == entry.kind.slug()) {
        return false;
    }
    if !m.path.is_empty()
        && !entry
            .provenance
            .first()
            .is_some_and(|p| p.path.starts_with(m.path.as_str()))
    {
        return false;
    }
    true
}

/// `(book, chapter, match)` indexes into the curated list.
type Place = (usize, usize, usize);
/// Specificity of a match: url length, path filter, kind filter.
type Score = (usize, bool, bool);

/// The most specific claim on the entry across all books, as `(book, chapter, match)`: the
/// longest url, then a path filter, then a kind filter. Equal specificity goes to the earlier
/// book, chapter, and match. The url decides, not the entry's `source`: upstream files
/// sometimes hold a few records of another publication.
fn best_match(books: &[BookSpec], entry: &RuleEntry) -> Option<Place> {
    let home = home_url(entry);
    let mut best: Option<(Score, Place)> = None;
    for (bi, book) in books.iter().enumerate() {
        for (ci, chapter) in book.chapter.iter().enumerate() {
            for (mi, m) in chapter.matches.iter().enumerate() {
                let score = (m.url.len(), !m.path.is_empty(), !m.kinds.is_empty());
                if matches(m, entry, home) && best.is_none_or(|(top, _)| score > top) {
                    best = Some((score, (bi, ci, mi)));
                }
            }
        }
    }
    best.map(|(_, at)| at)
}

struct Builder<'a> {
    by_id: BTreeMap<&'a EntryId, &'a RuleEntry>,
    links: BTreeMap<&'a EntryId, Vec<&'a NestedLink>>,
    placed: BTreeSet<&'a EntryId>,
}

impl<'a> Builder<'a> {
    fn node(&mut self, id: &'a EntryId) -> BookNode {
        self.placed.insert(id);
        let entry = self.by_id[id];
        let mut children: Vec<BookNode> = Vec::new();
        let links = self.links.get(id).cloned().unwrap_or_default();
        for link in links {
            if self.placed.contains(&link.target) {
                continue;
            }
            let target = self.by_id.get_key_value(&link.target).map(|(k, _)| *k);
            let Some(target) = target else { continue };
            let child = self.node(target);
            let mut level = &mut children;
            for (title, anchor) in &link.trail {
                let at = level
                    .iter()
                    .position(|n| n.entry.is_none() && n.anchor.as_deref() == Some(anchor));
                let at = at.unwrap_or_else(|| {
                    level.push(BookNode {
                        title: title.clone(),
                        entry: None,
                        anchor: Some(anchor.clone()),
                        review: None,
                        children: Vec::new(),
                    });
                    level.len() - 1
                });
                level = &mut level[at].children;
            }
            level.push(child);
        }
        BookNode {
            title: entry.title.clone(),
            entry: Some(id.clone()),
            anchor: None,
            review: None,
            children,
        }
    }
}

fn sort_key<'a>(entry: &'a RuleEntry, order: &UpstreamOrder) -> (u32, String, &'a EntryId) {
    (
        order.rank(home_url(entry)).unwrap_or(u32::MAX),
        entry
            .title
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect(),
        &entry.id,
    )
}

pub fn build(
    entries: &[RuleEntry],
    nested: &[NestedLink],
    spec: Option<&BookOrderFile>,
    order: &UpstreamOrder,
) -> Report {
    let by_id: BTreeMap<&EntryId, &RuleEntry> = entries.iter().map(|e| (&e.id, e)).collect();
    // A tree holds the entries that its text links to, even when an upstream record names
    // another publication as its source.
    let mut links: BTreeMap<&EntryId, Vec<&NestedLink>> = BTreeMap::new();
    let mut targets: BTreeSet<&EntryId> = BTreeSet::new();
    for link in nested {
        if let (Some((host, _)), Some((target, _))) = (
            by_id.get_key_value(&link.host),
            by_id.get_key_value(&link.target),
        ) && host != target
        {
            links.entry(host).or_default().push(link);
            targets.insert(target);
        }
    }
    let mut builder = Builder {
        by_id: by_id.clone(),
        links,
        placed: BTreeSet::new(),
    };

    let empty = BookOrderFile::default();
    let spec = spec.unwrap_or(&empty);
    let mut unplaced: Vec<Unplaced> = Vec::new();
    let no_claim = if spec.book.is_empty() {
        "book-order.toml is missing or lists no book"
    } else {
        "no chapter of the curated lists claims this url"
    };

    // First pass: entries that no tree nests are the tops of the chapters.
    // Second pass: entries that a tree should have nested but that stayed unplaced.
    let mut tops: BTreeMap<(usize, usize), Vec<&RuleEntry>> = BTreeMap::new();
    let mut leftovers: Vec<&RuleEntry> = Vec::new();
    for entry in entries {
        if targets.contains(&entry.id) {
            leftovers.push(entry);
        } else {
            match best_match(&spec.book, entry) {
                Some((bi, ci, _)) => tops.entry((bi, ci)).or_default().push(entry),
                None => unplaced.push(Unplaced {
                    entry: entry.id.to_string(),
                    reason: no_claim.into(),
                }),
            }
        }
    }
    let match_order = |entry: &RuleEntry| {
        best_match(&spec.book, entry)
            .map(|(_, _, mi)| mi)
            .unwrap_or(0)
    };
    let mut chapters: Vec<Vec<BookNode>> = spec
        .book
        .iter()
        .map(|b| {
            b.chapter
                .iter()
                .map(|c| BookNode {
                    title: c.title.clone(),
                    entry: None,
                    anchor: None,
                    review: c.review.clone(),
                    children: Vec::new(),
                })
                .collect()
        })
        .collect();
    for (&(bi, ci), list) in &mut tops {
        list.sort_by_key(|e| (match_order(e), sort_key(e, order)));
        for entry in list.iter() {
            let node = builder.node(&entry.id);
            chapters[bi][ci].children.push(node);
        }
    }
    let mut late: BTreeMap<(usize, usize), Vec<&RuleEntry>> = BTreeMap::new();
    for entry in leftovers {
        if builder.placed.contains(&entry.id) {
            continue;
        }
        match best_match(&spec.book, entry) {
            Some((bi, ci, _)) => late.entry((bi, ci)).or_default().push(entry),
            None => unplaced.push(Unplaced {
                entry: entry.id.to_string(),
                reason: format!(
                    "nested in an entry that no chapter claims, and: {}",
                    no_claim
                ),
            }),
        }
    }
    for ((bi, ci), mut list) in late {
        list.sort_by_key(|e| (match_order(e), sort_key(e, order)));
        for entry in list {
            if !builder.placed.contains(&entry.id) {
                let node = builder.node(&entry.id);
                chapters[bi][ci].children.push(node);
            }
        }
    }

    let mut trees = Vec::new();
    for (book, mut nodes) in spec.book.iter().zip(chapters) {
        for (node, chapter) in nodes.iter_mut().zip(&book.chapter) {
            promote_single_root(node, chapter, &by_id);
        }
        trees.push(BookTree {
            source: book.source.clone(),
            title: book.title.clone(),
            basis: book.basis.clone(),
            review: book.review.clone(),
            chapters: nodes,
        });
    }

    // Anything still outside every tree is reported once.
    let reported: BTreeSet<String> = unplaced.iter().map(|u| u.entry.clone()).collect();
    for entry in entries {
        if !builder.placed.contains(&entry.id) && !reported.contains(entry.id.as_str()) {
            unplaced.push(Unplaced {
                entry: entry.id.to_string(),
                reason: "its host entry is claimed by no chapter".into(),
            });
        }
    }
    unplaced.sort_by(|a, b| a.entry.cmp(&b.entry));
    let mut per_source: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
    for entry in entries {
        let counts = per_source.entry(entry.source_ids[0].as_str()).or_default();
        if builder.placed.contains(&entry.id) {
            counts.0 += 1;
        } else {
            counts.1 += 1;
        }
    }
    Report {
        index: BookIndex { books: trees },
        coverage: BookOrderCoverage {
            placed: builder.placed.len() as u32,
            unplaced,
            by_publication: per_source
                .into_iter()
                .map(|(source_id, (placed, unplaced))| BookPlacement {
                    source_id: source_id.to_string(),
                    placed,
                    unplaced,
                })
                .collect(),
        },
        stats: CanonicalBookStats::default(),
    }
}

/// Build the canonical (Pf Data, `ref/` id) book index from page attribution.
///
/// There is no curated chapter list for this corpus. Every publication that has
/// entries becomes one book. Its entries are ordered by the page that their
/// `‹SOURCE book/page›` line names, with one chapter for paged entries and one
/// for entries whose source line has no page. Page order is printed-order
/// evidence for the entry, not for chapter boundaries.
///
/// Sibling order is `(page, kind slug, normalized title, id)`. A child that a
/// [`NestedLink`] attaches to a host stays under that host, so it follows the
/// host even when its own page number is lower. Entries on the same page are
/// not in printed order. An entry with no page in any book is reported in
/// `coverage.unplaced`; nothing is dropped from the index.
pub fn build_canonical(
    entries: &[RuleEntry],
    nested: &[NestedLink],
    pages: &BTreeMap<EntryId, Vec<(String, Option<u32>)>>,
) -> Report {
    let by_id: BTreeMap<&EntryId, &RuleEntry> = entries.iter().map(|e| (&e.id, e)).collect();

    // Display titles come from the SOURCE lines themselves. The same publication
    // may be spelled with different case; the lexicographically first spelling
    // wins so two runs agree.
    let mut titles: BTreeMap<String, String> = BTreeMap::new();
    for rows in pages.values() {
        for (publication, _) in rows {
            let slug = slug_segment(publication);
            if slug.is_empty() {
                continue;
            }
            titles
                .entry(slug)
                .and_modify(|title| {
                    if publication < title {
                        *title = publication.clone();
                    }
                })
                .or_insert_with(|| publication.clone());
        }
    }

    let mut members: BTreeMap<String, BTreeMap<EntryId, Option<u32>>> = BTreeMap::new();
    let mut unplaced: Vec<Unplaced> = Vec::new();
    let mut placed_by_page = 0usize;
    let mut unpaged = 0usize;
    let mut multiple_books = 0usize;
    let mut no_publication = 0usize;
    let mut placements = 0usize;
    let mut paged_placements = 0usize;
    let mut unpaged_placements = 0usize;

    for entry in entries {
        let mut attribution: BTreeMap<String, Option<u32>> = BTreeMap::new();
        if let Some(rows) = pages.get(&entry.id) {
            for (publication, page) in rows {
                let slug = slug_segment(publication);
                if slug.is_empty() {
                    continue;
                }
                let slot = attribution.entry(slug).or_insert(*page);
                if let Some(page) = page {
                    *slot = Some(slot.map_or(*page, |earlier| earlier.min(*page)));
                }
            }
        }
        // A record's publication list can be wider than its SOURCE lines. Keep
        // that attribution as an unpaged membership rather than dropping it.
        for source in &entry.source_ids {
            attribution.entry(source.clone()).or_insert(None);
        }
        if attribution.is_empty() {
            no_publication += 1;
            unplaced.push(Unplaced {
                entry: entry.id.to_string(),
                reason: "no publication attribution; the entry cannot appear in any book".into(),
            });
            continue;
        }
        if attribution.values().any(Option::is_some) {
            placed_by_page += 1;
        } else {
            unpaged += 1;
            let books = attribution
                .keys()
                .map(|slug| titles.get(slug).map_or(slug.as_str(), String::as_str))
                .collect::<Vec<_>>()
                .join(", ");
            unplaced.push(Unplaced {
                entry: entry.id.to_string(),
                reason: format!(
                    "no page number in its source line; shown under {UNPAGED_CHAPTER} in {books}"
                ),
            });
        }
        if attribution.len() > 1 {
            multiple_books += 1;
        }
        placements += attribution.len();
        paged_placements += attribution.values().filter(|page| page.is_some()).count();
        unpaged_placements += attribution.values().filter(|page| page.is_none()).count();
        for (slug, page) in attribution {
            members
                .entry(slug)
                .or_default()
                .insert(entry.id.clone(), page);
        }
    }

    let mut books: Vec<BookTree> = Vec::new();
    let mut by_publication: Vec<BookPlacement> = Vec::new();
    for (source, book_members) in &members {
        let title = titles
            .get(source)
            .cloned()
            .unwrap_or_else(|| source.clone());

        // Nest only when both ends belong to this book. A cross-book link stays a
        // root in the other book instead of hiding the child from its own book.
        let mut children: BTreeMap<EntryId, Vec<EntryId>> = BTreeMap::new();
        let mut targets: BTreeSet<EntryId> = BTreeSet::new();
        let mut seen: BTreeSet<(&EntryId, &EntryId)> = BTreeSet::new();
        for link in nested {
            if link.host == link.target
                || !book_members.contains_key(&link.host)
                || !book_members.contains_key(&link.target)
                || !seen.insert((&link.host, &link.target))
            {
                continue;
            }
            children
                .entry(link.host.clone())
                .or_default()
                .push(link.target.clone());
            targets.insert(link.target.clone());
        }

        let mut roots: Vec<EntryId> = book_members
            .keys()
            .filter(|id| !targets.contains(*id))
            .cloned()
            .collect();
        sort_canonical(&mut roots, book_members, &by_id);
        let mut placed: BTreeSet<EntryId> = BTreeSet::new();
        let mut paged_nodes = Vec::new();
        let mut unpaged_nodes = Vec::new();
        for id in roots {
            let node = canonical_node(&id, book_members, &children, &by_id, &mut placed);
            if book_members[&id].is_some() {
                paged_nodes.push(node);
            } else {
                unpaged_nodes.push(node);
            }
        }
        // A cycle in the links must not hide its entries. The guard in
        // `canonical_node` stops the walk; anything unreached becomes a root.
        let mut leftovers: Vec<EntryId> = book_members
            .keys()
            .filter(|id| !placed.contains(*id))
            .cloned()
            .collect();
        sort_canonical(&mut leftovers, book_members, &by_id);
        for id in leftovers {
            let node = canonical_node(&id, book_members, &children, &by_id, &mut placed);
            unpaged_nodes.push(node);
        }

        let paged_count = book_members.values().filter(|page| page.is_some()).count();
        let unpaged_count = book_members.len() - paged_count;
        let mut chapters = Vec::new();
        if !paged_nodes.is_empty() {
            chapters.push(BookNode {
                title: PAGE_CHAPTER.into(),
                entry: None,
                anchor: None,
                review: Some(
                    "Entries follow their page number. Entries on the same page, and a child nested under its host, are not printed-order evidence."
                        .into(),
                ),
                children: paged_nodes,
            });
        }
        if !unpaged_nodes.is_empty() {
            chapters.push(BookNode {
                title: UNPAGED_CHAPTER.into(),
                entry: None,
                anchor: None,
                review: Some(
                    "These entries name this publication but their source line gives no page number, so their position is unknown."
                        .into(),
                ),
                children: unpaged_nodes,
            });
        }
        let basis = match (paged_count, unpaged_count) {
            (0, _) => {
                "No page number appears in this publication's Pf Data source lines, so the contents is one Unpaged list. Page order is evidence for entry order, not for chapter boundaries."
            }
            (_, 0) => {
                "Entries are ordered by the page each Pf Data record attributes to this publication. No printed chapter boundaries are known for this publication; this is a page-ordered list, not a chapter structure."
            }
            _ => {
                "Entries with a page are ordered by that page; entries without one follow in an Unpaged section. No printed chapter boundaries are known for this publication; this is a page-ordered list, not a chapter structure."
            }
        };
        books.push(BookTree {
            source: source.clone(),
            title,
            basis: basis.into(),
            review: vec![
                "Page attribution orders entries; it does not establish chapter boundaries or titles."
                    .into(),
                "Entries on the same page are not in printed order.".into(),
            ],
            chapters,
        });
        by_publication.push(BookPlacement {
            source_id: source.clone(),
            placed: paged_count as u32,
            unplaced: unpaged_count as u32,
        });
    }
    books.sort_by(|a, b| {
        title_key(&a.title)
            .cmp(&title_key(&b.title))
            .then_with(|| a.source.cmp(&b.source))
    });
    by_publication.sort_by(|a, b| a.source_id.cmp(&b.source_id));
    unplaced.sort_by(|a, b| a.entry.cmp(&b.entry));

    Report {
        index: BookIndex { books },
        coverage: BookOrderCoverage {
            placed: placed_by_page as u32,
            unplaced,
            by_publication,
        },
        stats: CanonicalBookStats {
            books: members.len(),
            entries: entries.len(),
            placed_by_page,
            unpaged,
            multiple_books,
            no_publication,
            placements,
            paged_placements,
            unpaged_placements,
        },
    }
}

fn title_key(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Sibling order inside one canonical book: page, then kind, then title, then id.
fn canonical_key<'a>(
    members: &BTreeMap<EntryId, Option<u32>>,
    by_id: &BTreeMap<&EntryId, &RuleEntry>,
    id: &'a EntryId,
) -> (u32, &'static str, String, &'a str) {
    let page = members.get(id).copied().flatten().unwrap_or(u32::MAX);
    let entry = by_id[id];
    (
        page,
        entry.kind.slug(),
        title_key(&entry.title),
        id.as_str(),
    )
}

fn sort_canonical(
    ids: &mut [EntryId],
    members: &BTreeMap<EntryId, Option<u32>>,
    by_id: &BTreeMap<&EntryId, &RuleEntry>,
) {
    ids.sort_by(|a, b| canonical_key(members, by_id, a).cmp(&canonical_key(members, by_id, b)));
}

fn canonical_node(
    id: &EntryId,
    members: &BTreeMap<EntryId, Option<u32>>,
    children: &BTreeMap<EntryId, Vec<EntryId>>,
    by_id: &BTreeMap<&EntryId, &RuleEntry>,
    placed: &mut BTreeSet<EntryId>,
) -> BookNode {
    placed.insert(id.clone());
    let mut kids: Vec<EntryId> = children.get(id).cloned().unwrap_or_default();
    kids.retain(|child| members.contains_key(child) && !placed.contains(child));
    sort_canonical(&mut kids, members, by_id);
    let children = kids
        .iter()
        .map(|child| canonical_node(child, members, children, by_id, placed))
        .collect();
    let entry = by_id[id];
    BookNode {
        title: entry.title.clone(),
        entry: Some(id.clone()),
        anchor: None,
        review: None,
        children,
    }
}

/// A chapter that holds one rules tree and nothing else shows that tree itself: the chapter
/// node opens the root entry and holds the root's sections. Otherwise the chapter only
/// groups its entries.
fn promote_single_root(
    chapter: &mut BookNode,
    spec: &ChapterSpec,
    by_id: &BTreeMap<&EntryId, &RuleEntry>,
) {
    let [single] = chapter.children.as_slice() else {
        return;
    };
    let Some(id) = &single.entry else { return };
    let entry = by_id[id];
    if entry.kind == EntryKind::Rule && spec.matches.iter().any(|m| home_url(entry) == m.url) {
        let node = chapter.children.remove(0);
        chapter.entry = node.entry;
        chapter.children = node.children;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{Provenance, TypedFields};

    fn entry(id: &str, title: &str, url: &str, kind: EntryKind) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: title.into(),
            source_ids: vec!["b".into()],
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::None,
            tags: vec![],
            links: vec![],
            provenance: vec![Provenance {
                dataset: "d".into(),
                record: url.into(),
                path: "p".into(),
            }],
            license_refs: vec!["ogl-1.0a".into()],
            relations: vec![],
        }
    }

    fn spec() -> BookOrderFile {
        toml::from_str(
            r#"
[[book]]
source = "b"
title = "B"
[[book.chapter]]
title = "Races"
match = [{ url = "pfsrd://B/Races" }]
[[book.chapter]]
title = "Combat"
match = [{ url = "pfsrd://B/Rules/Combat" }]
"#,
        )
        .unwrap()
    }

    fn link(host: &str, target: &str, trail: &[(&str, &str)]) -> NestedLink {
        NestedLink {
            host: EntryId::parse(host).unwrap(),
            target: EntryId::parse(target).unwrap(),
            trail: trail
                .iter()
                .map(|(t, a)| (t.to_string(), a.to_string()))
                .collect(),
            child: false,
        }
    }

    #[test]
    fn nested_entries_sit_under_their_heading_and_roots_become_chapters() {
        let entries = vec![
            entry(
                "b/races/dwarves",
                "Dwarves",
                "pfsrd://B/Races/Dwarves",
                EntryKind::Race,
            ),
            entry(
                "b/races/dwarves/hardy",
                "Hardy",
                "pfsrd://B/Races/Dwarves/Traits/Hardy",
                EntryKind::RacialTrait,
            ),
            entry(
                "b/rules/combat",
                "Combat",
                "pfsrd://B/Rules/Combat",
                EntryKind::Rule,
            ),
            entry(
                "b/rules/combat/actions",
                "Actions",
                "pfsrd://B/Rules/Combat/Actions",
                EntryKind::Rule,
            ),
            entry(
                "b/races/elves",
                "Elves",
                "pfsrd://B/Races/Elves",
                EntryKind::Race,
            ),
        ];
        let nested = vec![
            link(
                "b/races/dwarves",
                "b/races/dwarves/hardy",
                &[("Dwarf Racial Traits", "dwarf-racial-traits")],
            ),
            link("b/rules/combat", "b/rules/combat/actions", &[]),
        ];
        let order = UpstreamOrder::parse(
            r#"{"lists":[{"parent":"pfsrd://B/Races","children":["Elves","Dwarves"]}]}"#,
        )
        .unwrap();
        let report = build(&entries, &nested, Some(&spec()), &order);
        assert!(
            report.coverage.unplaced.is_empty(),
            "{:?}",
            report.coverage.unplaced
        );
        assert_eq!(report.coverage.placed, 5);
        let tree = &report.index.books[0];
        let races = &tree.chapters[0];
        let titles: Vec<&str> = races.children.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Elves", "Dwarves"],
            "upstream order, not alphabetical"
        );
        let container = &races.children[1].children[0];
        assert_eq!(container.entry, None);
        assert_eq!(container.anchor.as_deref(), Some("dwarf-racial-traits"));
        assert_eq!(container.children[0].title, "Hardy");
        let combat = &tree.chapters[1];
        assert_eq!(combat.entry.as_ref().unwrap().as_str(), "b/rules/combat");
        assert_eq!(combat.children[0].title, "Actions");
    }

    #[test]
    fn unclaimed_entries_are_reported_never_dropped() {
        let entries = vec![entry(
            "b/other/x",
            "X",
            "pfsrd://B/Other/X",
            EntryKind::Rule,
        )];
        let report = build(&entries, &[], Some(&spec()), &UpstreamOrder::default());
        assert_eq!(report.coverage.unplaced.len(), 1);
        assert_eq!(report.coverage.placed, 0);
        let none = build(&entries, &[], None, &UpstreamOrder::default());
        assert!(none.coverage.unplaced[0].reason.contains("missing"));
        assert!(none.index.books.is_empty());
    }

    #[test]
    fn the_most_specific_match_wins_and_kinds_split_a_url() {
        let book: BookOrderFile = toml::from_str(
            r#"
[[book]]
source = "b"
title = "B"
[[book.chapter]]
title = "Classes"
match = [{ url = "pfsrd://B/Classes", kinds = ["class"] }]
[[book.chapter]]
title = "Prestige"
match = [{ url = "pfsrd://B/Classes", kinds = ["prestige_class"] }]
"#,
        )
        .unwrap();
        let entries = vec![
            entry(
                "b/classes/fighter",
                "Fighter",
                "pfsrd://B/Classes/Fighter",
                EntryKind::Class,
            ),
            entry(
                "b/classes/duelist",
                "Duelist",
                "pfsrd://B/Classes/Duelist",
                EntryKind::PrestigeClass,
            ),
        ];
        let report = build(&entries, &[], Some(&book), &UpstreamOrder::default());
        let chapters = &report.index.books[0].chapters;
        assert_eq!(chapters[0].children[0].title, "Fighter");
        assert_eq!(chapters[1].children[0].title, "Duelist");
    }

    fn canonical_entry(id: &str, title: &str, kind: EntryKind, sources: &[&str]) -> RuleEntry {
        let mut entry = entry(id, title, "", kind);
        entry.source_ids = sources.iter().map(|source| source.to_string()).collect();
        entry
    }

    fn page_map(
        rows: &[(&str, &str, Option<u32>)],
    ) -> BTreeMap<EntryId, Vec<(String, Option<u32>)>> {
        let mut pages: BTreeMap<EntryId, Vec<(String, Option<u32>)>> = BTreeMap::new();
        for (id, publication, page) in rows {
            pages
                .entry(EntryId::parse(*id).unwrap())
                .or_default()
                .push(((*publication).to_string(), *page));
        }
        pages
    }

    fn titles_of(nodes: &[BookNode]) -> Vec<&str> {
        nodes.iter().map(|node| node.title.as_str()).collect()
    }

    fn flatten_titles(node: &BookNode, out: &mut Vec<String>) {
        if let Some(entry) = &node.entry {
            out.push(entry.to_string());
        }
        for child in &node.children {
            flatten_titles(child, out);
        }
    }

    #[test]
    fn canonical_books_order_by_page_then_ties_and_keep_unpaged_entries() {
        let entries = vec![
            canonical_entry("b/zeta", "Zeta", EntryKind::Rule, &["book-a"]),
            canonical_entry("b/beta", "Beta", EntryKind::Rule, &["book-a"]),
            canonical_entry("b/alpha", "Alpha", EntryKind::Rule, &["book-a"]),
            canonical_entry("b/gamma", "Gamma", EntryKind::Rule, &["book-a"]),
        ];
        let pages = page_map(&[
            ("b/zeta", "Book A", Some(10)),
            ("b/beta", "Book A", Some(20)),
            ("b/alpha", "Book A", Some(20)),
        ]);
        let report = build_canonical(&entries, &[], &pages);
        assert_eq!(report.stats.books, 1);
        let book = &report.index.books[0];
        assert_eq!(book.title, "Book A");
        assert_eq!(book.chapters[0].title, PAGE_CHAPTER);
        assert_eq!(
            titles_of(&book.chapters[0].children),
            ["Zeta", "Alpha", "Beta"],
            "page order, then title inside one page"
        );
        assert_eq!(book.chapters[1].title, UNPAGED_CHAPTER);
        assert_eq!(titles_of(&book.chapters[1].children), ["Gamma"]);
        assert_eq!(report.coverage.placed, 3);
        assert_eq!(report.stats.unpaged, 1);
        assert_eq!(report.coverage.unplaced.len(), 1);
        assert!(report.coverage.unplaced[0].reason.contains(UNPAGED_CHAPTER));
    }

    #[test]
    fn canonical_ties_use_kind_then_title_then_id() {
        let entries = vec![
            canonical_entry("b/spell", "Alpha", EntryKind::Spell, &["book-a"]),
            canonical_entry("b/feat-zulu", "Zulu", EntryKind::Feat, &["book-a"]),
            canonical_entry("b/feat-alpha-2", "Alpha", EntryKind::Feat, &["book-a"]),
            canonical_entry("b/feat-alpha-1", "Alpha", EntryKind::Feat, &["book-a"]),
        ];
        let pages = page_map(&[
            ("b/spell", "Book A", Some(5)),
            ("b/feat-zulu", "Book A", Some(5)),
            ("b/feat-alpha-2", "Book A", Some(5)),
            ("b/feat-alpha-1", "Book A", Some(5)),
        ]);
        let report = build_canonical(&entries, &[], &pages);
        assert_eq!(
            titles_of(&report.index.books[0].chapters[0].children),
            ["Alpha", "Alpha", "Zulu", "Alpha"]
        );
        let ids: Vec<&str> = report.index.books[0].chapters[0]
            .children
            .iter()
            .filter_map(|node| node.entry.as_ref().map(EntryId::as_str))
            .collect();
        assert_eq!(
            ids,
            ["b/feat-alpha-1", "b/feat-alpha-2", "b/feat-zulu", "b/spell"],
            "kind slug, then normalized title, then id"
        );
    }

    #[test]
    fn a_nested_child_follows_its_host_instead_of_its_own_page() {
        let entries = vec![
            canonical_entry("b/earlier", "Earlier", EntryKind::Rule, &["book-a"]),
            canonical_entry("b/host", "Host", EntryKind::Rule, &["book-a"]),
            canonical_entry("b/child", "Child", EntryKind::Rule, &["book-a"]),
        ];
        let pages = page_map(&[
            ("b/earlier", "Book A", Some(10)),
            ("b/host", "Book A", Some(20)),
            ("b/child", "Book A", Some(5)),
        ]);
        let nested = vec![link("b/host", "b/child", &[])];
        let report = build_canonical(&entries, &nested, &pages);
        let paged = &report.index.books[0].chapters[0];
        assert_eq!(titles_of(&paged.children), ["Earlier", "Host"]);
        assert_eq!(titles_of(&paged.children[1].children), ["Child"]);
        let mut flat = Vec::new();
        flatten_titles(&report.index.books[0].chapters[0], &mut flat);
        assert_eq!(flat, ["b/earlier", "b/host", "b/child"]);
        assert_eq!(report.stats.placed_by_page, 3);
    }

    #[test]
    fn an_entry_can_hold_a_place_in_two_books() {
        let entries = vec![
            canonical_entry("b/shared", "Shared", EntryKind::Rule, &["book-a"]),
            canonical_entry("b/other", "Other", EntryKind::Rule, &["book-b"]),
        ];
        let pages = page_map(&[
            ("b/shared", "Book A", Some(10)),
            ("b/shared", "Book B", None),
            ("b/other", "Book B", Some(20)),
        ]);
        let report = build_canonical(&entries, &[], &pages);
        assert_eq!(report.stats.books, 2);
        assert_eq!(report.stats.multiple_books, 1);
        assert_eq!(report.stats.placements, 3);
        let a = report
            .index
            .books
            .iter()
            .find(|b| b.title == "Book A")
            .unwrap();
        let b = report
            .index
            .books
            .iter()
            .find(|b| b.title == "Book B")
            .unwrap();
        assert_eq!(titles_of(&a.chapters[0].children), ["Shared"]);
        assert_eq!(titles_of(&b.chapters[0].children), ["Other"]);
        assert_eq!(titles_of(&b.chapters[1].children), ["Shared"]);
        let b_coverage = report
            .coverage
            .by_publication
            .iter()
            .find(|row| row.source_id == "book-b")
            .unwrap();
        assert_eq!((b_coverage.placed, b_coverage.unplaced), (1, 1));
        assert!(
            report.coverage.unplaced.is_empty(),
            "the entry has a page in Book A, so it is not globally unplaced"
        );
    }

    #[test]
    fn unattributed_entries_are_reported_not_dropped() {
        let entries = vec![canonical_entry("b/orphan", "Orphan", EntryKind::Rule, &[])];
        let report = build_canonical(&entries, &[], &BTreeMap::new());
        assert!(report.index.books.is_empty());
        assert_eq!(report.stats.no_publication, 1);
        assert_eq!(report.coverage.unplaced.len(), 1);
        assert!(
            report.coverage.unplaced[0]
                .reason
                .contains("no publication")
        );
    }

    #[test]
    fn canonical_book_output_is_deterministic() {
        let entries = vec![
            canonical_entry("b/b", "B", EntryKind::Rule, &["book-a"]),
            canonical_entry("b/a", "A", EntryKind::Rule, &["book-a"]),
        ];
        let pages = page_map(&[("b/a", "Book A", Some(1)), ("b/b", "Book A", Some(1))]);
        let first = build_canonical(&entries, &[], &pages);
        let second = build_canonical(&entries, &[], &pages);
        assert_eq!(first.index, second.index);
        assert_eq!(first.coverage, second.coverage);
        assert_eq!(first.stats, second.stats);
    }

    #[test]
    fn real_corpus_core_rulebook_orders_magic_after_combat() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dataset = root.join(".cache/sources/pathfinder-data-1-e");
        if !dataset.join("json").is_dir() {
            eprintln!("skipping: the pinned primary source is not present");
            return;
        }
        let inventory = crate::pf_data::inventory(
            &dataset,
            &root.join("sources/pathfinder-data-1-e/source.json"),
        )
        .unwrap();
        let allowlist = crate::pf_rules::RuleAllowlist::load_optional(
            &root.join("sources/pathfinder-data-1-e/derived-rules.json"),
        )
        .unwrap();
        let converted = crate::canonical::convert_corpus(&inventory, &allowlist).unwrap();
        let report = build_canonical(&converted.entries, &converted.links, &converted.pages);
        let again = build_canonical(&converted.entries, &converted.links, &converted.pages);
        assert_eq!(
            serde_json::to_vec(&report.index).unwrap(),
            serde_json::to_vec(&again.index).unwrap(),
            "two runs over one conversion are byte-identical"
        );

        let core = report
            .index
            .books
            .iter()
            .find(|book| book.title == crate::pf_derived::CORE_RULEBOOK)
            .expect("the Core Rulebook has a canonical book");
        let mut flat = Vec::new();
        for chapter in &core.chapters {
            flatten_titles(chapter, &mut flat);
        }
        let position: BTreeMap<&str, usize> = flat
            .iter()
            .enumerate()
            .map(|(index, id)| (id.as_str(), index))
            .collect();
        let page_of = |title: &str, page: u32| {
            converted
                .entries
                .iter()
                .find(|entry| {
                    entry.title == title
                        && converted.pages.get(&entry.id).is_some_and(|rows| {
                            rows.iter().any(|(publication, at)| {
                                publication == crate::pf_derived::CORE_RULEBOOK && *at == Some(page)
                            })
                        })
                })
                .unwrap_or_else(|| panic!("{title} at page {page} was not converted"))
        };
        let combat = page_of("How Combat Works", 178);
        let magic = page_of("Casting Spells", 206);
        let combat_at = position[combat.id.as_str()];
        let magic_at = position[magic.id.as_str()];
        assert!(
            magic_at > combat_at,
            "Magic at page 206 must follow Combat at page 178: {combat_at} -> {magic_at}"
        );
        eprintln!(
            "canonical books: {} books, {} entries, {} placed by page, {} unpaged, {} in multiple books, {} without publication, {} placements",
            report.stats.books,
            report.stats.entries,
            report.stats.placed_by_page,
            report.stats.unpaged,
            report.stats.multiple_books,
            report.stats.no_publication,
            report.stats.placements
        );
    }
}
