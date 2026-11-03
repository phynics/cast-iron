//! The book view: each publication's chapters in the original order.
//!
//! Chapters and their order come from the curated `book-order.toml`. Inside a chapter, an
//! entry sits where the rendered text puts it: a rules section under its rules tree, a
//! racial trait under its race, an archetype under the class section that holds it. Records
//! that no tree nests, such as spells and feats, are ordered by the upstream sibling order.
//! Nothing is sorted alphabetically unless the upstream order itself is alphabetical.

use crate::build::NestedLink;
use crate::order::{BookOrderFile, BookSpec, ChapterSpec, MatchSpec, UpstreamOrder};
use cast_iron_schema::{
    BookIndex, BookNode, BookOrderCoverage, BookPlacement, BookTree, EntryId, EntryKind, RuleEntry,
    Unplaced,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Report {
    pub index: BookIndex,
    pub coverage: BookOrderCoverage,
}

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
}
