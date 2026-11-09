//! Small in-memory corpora for tests.

use crate::config::Config;
use crate::entry::{derived_links, entry_path, write_entry};
use crate::files::{Exclusions, Publication};
use crate::load::{Corpus, Stored};
use cast_iron_schema::{
    Block, BookIndex, EntryId, EntryKind, Inline, LegalNotices, LicenseText, ParagraphStyle,
    RuleEntry, TypedFields,
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
        aon: None,
        source_ids: vec!["core-rulebook".into()],
        sources: vec![],
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
    Corpus {
        dir: PathBuf::from("corpus"),
        config: Config::default(),
        entries: by_id,
        publications: vec![Publication {
            id: "core-rulebook".into(),
            title: "Core Rulebook".into(),
            aon: None,
            copyright: Some("Core Rulebook, (c) Paizo Inc.".into()),
            category: None,
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
            suffix: None,
        }],
        option_groups: Default::default(),
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
            section_15: vec![cast_iron_schema::CopyrightNotice {
                title: "Core Rulebook".into(),
                notice: "Core Rulebook, (c) Paizo Inc.".into(),
            }],
            notice_pending: vec![],
            datasets: vec![],
        },
        exclusions: Exclusions::default(),
        history: None,
        aliases: None,
    }
}
