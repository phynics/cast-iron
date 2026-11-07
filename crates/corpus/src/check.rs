//! Cross-file checks over a loaded corpus.

use crate::drift::{Disagreement, find_drift};
use crate::entry::entry_path;
use crate::load::Corpus;
use cast_iron_schema::headings::keyed_headings;
use cast_iron_schema::{Block, BookNode, EntryKind, RuleEntry, TypedFields};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

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

pub fn fields_kind(fields: &TypedFields) -> &'static str {
    match fields {
        TypedFields::Spell(_) => "spell",
        TypedFields::Feat(_) => "feat",
        TypedFields::Creature(_) => "creature",
        TypedFields::Item(_) => "item",
        TypedFields::Class(_) => "class",
        TypedFields::Trait(_) => "trait",
        TypedFields::RacialTrait(_) => "racial_trait",
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
        let at = |needle: &str| format!("{}:{}", path.display(), line_of(&stored.text, needle));
        if !fields_match(entry.kind, &entry.fields) {
            report.problems.push(format!(
                "{}: entry kind `{}` does not match `[fields]` kind `{}`",
                at("kind"),
                entry.kind.slug(),
                fields_kind(&entry.fields)
            ));
        }
        for (line, token) in upstream_markup_tokens(&stored.text) {
            report.problems.push(format!(
                "{}:{line}: left-over upstream token `{token}`. Convert it to plain text or a typed link.",
                path.display()
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
    use cast_iron_schema::{BookIndex, BookNode, BookTree, EntryId, Inline};

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
}
