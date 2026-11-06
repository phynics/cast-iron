//! Small in-memory corpora for tests.

use crate::config::Config;
use crate::entry::{derived_links, entry_path, write_entry};
use crate::files::Publication;
use crate::load::{Corpus, Stored};
use cast_iron_schema::{
    Block, BookIndex, CoverageReport, EntryId, EntryKind, Inline, LegalNotices, LicenseText,
    ParagraphStyle, RuleEntry, TypedFields,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn paragraph(content: Vec<Inline>) -> Block {
    Block::Paragraph {
        style: ParagraphStyle::Normal,
        content,
    }
}

pub fn heading(anchor: Option<&str>, text: &str) -> Block {
    Block::Heading {
        level: 2,
        anchor: anchor.map(str::to_string),
        content: vec![Inline::text(text)],
    }
}

pub fn link(target: &str, anchor: Option<&str>, label: &str) -> Inline {
    Inline::Link {
        target: EntryId::parse(target).unwrap(),
        anchor: anchor.map(str::to_string),
        c: vec![Inline::text(label)],
    }
}

/// A `rule` entry in publication `core-rulebook` with the default licences.
pub fn rule(id: &str, body: Vec<Block>) -> RuleEntry {
    let mut entry = RuleEntry {
        id: EntryId::parse(id).unwrap(),
        kind: EntryKind::Rule,
        title: id.rsplit('/').next().unwrap().to_string(),
        source_ids: vec!["core-rulebook".into()],
        section: vec![],
        parent: None,
        body,
        fields: TypedFields::None,
        tags: vec![],
        links: vec![],
        provenance: vec![],
        license_refs: vec!["ogl-1.0a".into(), "legal#community-use".into()],
        relations: vec![],
    };
    entry.links = derived_links(&entry.body);
    entry
}

/// A corpus holding `entries`, one publication, and one licence.
pub fn corpus(entries: Vec<RuleEntry>) -> Corpus {
    let mut by_id = BTreeMap::new();
    for entry in entries {
        let text = write_entry(&entry, &Config::default());
        by_id.insert(
            entry.id.to_string(),
            Stored {
                path: entry_path(&entry.id),
                text,
                entry,
            },
        );
    }
    let mut by_kind: BTreeMap<String, u32> = BTreeMap::new();
    for stored in by_id.values() {
        *by_kind
            .entry(stored.entry.kind.slug().to_string())
            .or_default() += 1;
    }
    Corpus {
        dir: PathBuf::from("corpus"),
        config: Config::default(),
        entries: by_id,
        publications: vec![Publication {
            id: "core-rulebook".into(),
            title: "Core Rulebook".into(),
            copyright: None,
            category: None,
            publication_date: None,
        }],
        books: BookIndex { books: vec![] },
        books_text: String::new(),
        legal: LegalNotices {
            licenses: vec![LicenseText {
                id: "ogl-1.0a".into(),
                title: "Open Game License".into(),
                text: "text".into(),
            }],
            product_identity: String::new(),
            community_use_notice: "notice".into(),
            section_15: vec![],
            notice_pending: vec![],
            datasets: vec![],
        },
        coverage: CoverageReport {
            input_digest: "0".repeat(64),
            by_kind,
            ..Default::default()
        },
        aliases: None,
    }
}
