//! `migrate`: convert a corpus from an older format to the current one.
//!
//! A format change (see `docs/corpus-format.md`) adds one step to [`MIGRATIONS`]. The
//! step rewrites the files of a corpus at format N into format N + 1 and changes
//! nothing else, so one mechanical pull request can apply it, followed by `fmt`.
//! Format 1 to 2 lowers the trades of archetypes and class options (`migrate_trades`). Format 2
//! to 3 (#425) splits `books.toml` into `books/<publication>.toml`.

use crate::config::{CORPUS_FORMAT, MIGRATE_COMMAND};
use std::fs;
use std::path::Path;

/// One step: the format it reads and the function that rewrites the corpus in
/// place, including `corpus.toml`'s `format` line.
pub struct Migration {
    pub from: u32,
    pub apply: fn(&Path) -> Result<(), String>,
}

/// The registered steps, oldest first: 1 to 2 (trades), then 2 to 3 (book layout).
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        from: 1,
        apply: lower_trades,
    },
    Migration {
        from: 2,
        apply: crate::layout::migrate_to_split,
    },
];

/// Format 1 to 2: the trades. `cast-iron-corpus migrate-trades` also writes the review queue.
fn lower_trades(dir: &Path) -> Result<(), String> {
    crate::migrate_trades::migrate_trades(dir).map(|_| ())
}

/// The `format` number in `corpus.toml`.
fn declared_format(dir: &Path) -> Result<u32, String> {
    let path = dir.join("corpus.toml");
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let value: toml::Table =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    value
        .get("format")
        .and_then(toml::Value::as_integer)
        .and_then(|format| u32::try_from(format).ok())
        .ok_or_else(|| format!("{}: no `format` number", path.display()))
}

/// Runs every step from the corpus's format up to `target` and returns a summary.
pub fn migrate_with(dir: &Path, target: u32, steps: &[Migration]) -> Result<String, String> {
    let mut format = declared_format(dir)?;
    if format == target {
        return Ok(format!(
            "{} is already at format {target}; nothing to migrate",
            dir.display()
        ));
    }
    if format > target {
        return Err(format!(
            "{} is at format {format}, newer than this tool's format {target}; update the tool",
            dir.display()
        ));
    }
    let start = format;
    while format < target {
        let step = steps.iter().find(|step| step.from == format).ok_or_else(|| {
            format!(
                "no migration from format {format} is registered in this tool; use the release that supports it"
            )
        })?;
        (step.apply)(dir)?;
        let now = declared_format(dir)?;
        if now <= format {
            return Err(format!(
                "the migration from format {format} did not advance `format` in corpus.toml"
            ));
        }
        format = now;
    }
    Ok(format!(
        "migrated {} from format {start} to {target}; now run `{} fmt .` and commit the result as one change",
        dir.display(),
        MIGRATE_COMMAND.trim_end_matches(" migrate")
    ))
}

/// Migrates to [`CORPUS_FORMAT`].
pub fn migrate(dir: &Path) -> Result<String, String> {
    migrate_with(dir, CORPUS_FORMAT, MIGRATIONS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_corpus(name: &str, format: u32) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cast-iron-migrate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("corpus.toml"),
            format!("format = {format}\n\n[defaults]\nlicense_refs = []\n"),
        )
        .unwrap();
        dir
    }

    fn bump(dir: &Path) -> Result<(), String> {
        let text = fs::read_to_string(dir.join("corpus.toml")).unwrap();
        fs::write(
            dir.join("corpus.toml"),
            text.replace("format = 0", "format = 1"),
        )
        .unwrap();
        Ok(())
    }

    #[test]
    fn a_current_corpus_has_nothing_to_migrate() {
        let dir = temp_corpus("current", CORPUS_FORMAT);
        let message = migrate(&dir).unwrap();
        assert!(message.contains("nothing to migrate"), "{message}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_registered_step_runs_and_advances_the_format() {
        let dir = temp_corpus("step", 0);
        let steps = [Migration {
            from: 0,
            apply: bump,
        }];
        let message = migrate_with(&dir, 1, &steps).unwrap();
        assert!(message.contains("from format 0 to 1"), "{message}");
        assert_eq!(declared_format(&dir).unwrap(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_step_a_newer_corpus_and_a_stuck_step_are_errors() {
        let dir = temp_corpus("missing", 0);
        let error = migrate_with(&dir, 1, &[]).unwrap_err();
        assert!(error.contains("no migration from format 0"), "{error}");
        let stuck = [Migration {
            from: 0,
            apply: |_| Ok(()),
        }];
        let error = migrate_with(&dir, 1, &stuck).unwrap_err();
        assert!(error.contains("did not advance"), "{error}");
        let _ = fs::remove_dir_all(&dir);
        let dir = temp_corpus("newer", 7);
        let error = migrate(&dir).unwrap_err();
        assert!(error.contains("newer than this tool"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_format_1_corpus_migrates_through_both_steps() {
        use crate::entry::{entry_path, write_entry};
        use crate::testkit::{corpus_in_format, heading, rule};
        use cast_iron_schema::{
            ArchetypeFields, EntryId, EntryKind, Relation, RelationKind, Trade, TradeOp,
            TypedFields,
        };
        let dir = temp_corpus("chain", 1);
        // A fighter with a `bravery` heading, and an archetype that replaces it: written in
        // format 1 as two parallel lists, which the trades step lowers to one resolved trade.
        let mut fighter = rule(
            "ref/class/fighter",
            vec![heading(Some("bravery"), "Bravery (Ex)")],
        );
        fighter.kind = EntryKind::Class;
        fighter.fields = TypedFields::Class(Default::default());
        let mut scout = rule("ref/archetype/scout", vec![]);
        scout.kind = EntryKind::Archetype;
        scout.fields = TypedFields::Archetype(ArchetypeFields {
            trades: vec![Trade::unresolved(TradeOp::Replaces, "Bravery (Ex)")],
            ..Default::default()
        });
        scout.relations = vec![Relation {
            kind: RelationKind::Archetype,
            target: EntryId::parse("ref/class/fighter").unwrap(),
            label: None,
        }];
        let source = corpus_in_format(1, vec![rule("ref/rule/a", vec![]), fighter, scout]);
        for stored in source.entries.values() {
            let path = dir.join(entry_path(&stored.entry.id));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, write_entry(&stored.entry, &source.config)).unwrap();
        }
        let write = |name: &str, text: String| fs::write(dir.join(name), text).unwrap();
        write(
            "publications.toml",
            "[[publication]]\nid = \"core-rulebook\"\ntitle = \"Core Rulebook\"\ncopyright = \"c\"\n"
                .into(),
        );
        write("legal.toml", crate::files::write_legal(&source.legal));
        write(
            "exclusions.toml",
            crate::files::write_exclusions(&source.exclusions),
        );
        // One curated book and its stored catch-all chapter, as the pinned corpus has them.
        let chapter = |title: &str| cast_iron_schema::BookNode {
            title: title.into(),
            entry: None,
            anchor: None,
            review: None,
            collapse: false,
            lists_children: false,
            children: vec![],
        };
        write(
            "books.toml",
            crate::files::write_books(&cast_iron_schema::BookIndex {
                books: vec![cast_iron_schema::BookTree {
                    source: "core-rulebook".into(),
                    title: "Core Rulebook".into(),
                    basis: String::new(),
                    review: vec![],
                    chapters: vec![chapter("Introduction"), chapter("New entries (page order)")],
                }],
            }),
        );

        assert!(
            fs::read_to_string(dir.join("entries/archetype/scout.md"))
                .unwrap()
                .contains("replaces = [\"Bravery (Ex)\"]"),
            "the fixture is format 1"
        );

        // The registered chain: 1 to 2 (trades), then 2 to 3 (layout).
        let message = migrate(&dir).unwrap();
        assert!(message.contains("from format 1 to 3"), "{message}");
        assert_eq!(declared_format(&dir).unwrap(), 3);
        assert!(!dir.join("books.toml").exists());
        let loaded = crate::load::Corpus::load(&dir).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(loaded.books.books.len(), 1);
        // The stored catch-all is gone; the compiler regenerates it.
        assert_eq!(loaded.books.books[0].chapters.len(), 1);
        assert_eq!(loaded.book_index().books[0].chapters.len(), 2);
        // The trades step ran: the archetype holds one flat, resolved trade.
        let TypedFields::Archetype(fields) = &loaded.entries["ref/archetype/scout"].entry.fields
        else {
            panic!("an archetype");
        };
        assert_eq!(fields.trades.len(), 1);
        assert_eq!(fields.trades[0].feature.as_ref().unwrap().anchor, "bravery");
        assert!(
            fs::read_to_string(dir.join("entries/archetype/scout.md"))
                .unwrap()
                .contains("trades = ["),
        );
        // A second run has nothing to do.
        let again = migrate(&dir).unwrap();
        assert!(again.contains("nothing to migrate"), "{again}");
        let _ = fs::remove_dir_all(&dir);
    }
}
