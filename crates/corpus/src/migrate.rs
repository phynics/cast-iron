//! `migrate`: convert a corpus from an older format to the current one.
//!
//! A format change (see `docs/corpus-format.md`) adds one step to [`MIGRATIONS`]. The
//! step rewrites the files of a corpus at format N into format N + 1 and changes
//! nothing else, so one mechanical pull request can apply it, followed by `fmt`.
//! No migration exists yet: the corpus is at its first format.

use crate::config::{CORPUS_FORMAT, MIGRATE_COMMAND};
use std::fs;
use std::path::Path;

/// One step: the format it reads and the function that rewrites the corpus in
/// place, including `corpus.toml`'s `format` line.
pub struct Migration {
    pub from: u32,
    pub apply: fn(&Path) -> Result<(), String>,
}

/// The registered steps, oldest first. Empty while the corpus is at format 1.
pub const MIGRATIONS: &[Migration] = &[];

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
}
