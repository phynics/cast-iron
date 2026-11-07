//! The contributor kit's examples must stay valid: they are what a contributor copies.

use cast_iron_corpus::{
    Config, Corpus, Publication, Stored, check, entry_path, read_entry, write_entry,
};
use cast_iron_schema::{BookIndex, EntryId, EntryKind, LegalNotices, LicenseText};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus-kit/examples")
}

/// The example without the comment lines of its front matter.
fn without_comments(text: &str) -> String {
    let mut in_front = false;
    let mut fences = 0;
    let mut out = Vec::new();
    for line in text.lines() {
        if line == "+++" {
            fences += 1;
            in_front = fences == 1;
        } else if in_front && line.starts_with('#') {
            continue;
        }
        out.push(line);
    }
    let mut joined = out.join("\n");
    joined.push('\n');
    joined
}

fn id_of(text: &str) -> EntryId {
    let line = text
        .lines()
        .find(|line| line.starts_with("id = "))
        .expect("id line");
    EntryId::parse(line.trim_start_matches("id = ").trim_matches('"')).unwrap()
}

fn load_examples() -> Vec<(String, String, cast_iron_schema::RuleEntry)> {
    let mut found = Vec::new();
    for item in fs::read_dir(examples_dir()).unwrap() {
        let path = item.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let stripped = without_comments(&fs::read_to_string(&path).unwrap());
        let id = id_of(&stripped);
        let entry = read_entry(&entry_path(&id), &stripped, &Config::default())
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        found.push((name, stripped, entry));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[test]
fn every_entry_kind_has_an_example() {
    let kinds: BTreeSet<EntryKind> = load_examples()
        .iter()
        .map(|(_, _, entry)| entry.kind)
        .collect();
    for kind in EntryKind::ALL {
        assert!(kinds.contains(&kind), "no example for {}", kind.slug());
    }
}

#[test]
fn each_example_is_canonical_once_its_comments_are_removed() {
    for (name, stripped, entry) in load_examples() {
        assert_eq!(
            write_entry(&entry, &Config::default()),
            stripped,
            "{name} is not in canonical form; run `cast-iron-corpus fmt` on a copy"
        );
    }
}

#[test]
fn the_examples_check_as_one_corpus_with_no_drift() {
    let mut entries = BTreeMap::new();
    for (_, text, entry) in load_examples() {
        entries.insert(
            entry.id.to_string(),
            Stored {
                path: entry_path(&entry.id),
                text,
                entry,
            },
        );
    }
    let corpus = Corpus {
        dir: PathBuf::from("corpus-kit"),
        config: Config::default(),
        entries,
        publications: vec![Publication {
            id: "example-book".into(),
            title: "Example Book".into(),
            copyright: Some("Example Book, (c) Example Publisher.".into()),
            category: None,
            publication_date: None,
            suffix: None,
        }],
        option_groups: {
            let mut groups = cast_iron_corpus::OptionGroups::default();
            groups.groups.insert(
                "arcane-discovery".into(),
                cast_iron_corpus::OptionGroupDef {
                    label: "Arcane discoveries".into(),
                    singular: "Arcane discovery".into(),
                    classes: vec!["ref/class/example-wayfarer".into()],
                },
            );
            groups
        },
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
                title: "Example Book".into(),
                notice: "Example Book, (c) Example Publisher.".into(),
            }],
            notice_pending: vec![],
            datasets: vec![],
        },
        exclusions: Default::default(),
        history: None,
        aliases: None,
    };
    let report = check(&corpus);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert!(report.drift.is_empty(), "{:?}", report.problems);
}
