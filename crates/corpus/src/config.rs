//! `corpus.toml`: the corpus format version and the defaults entries omit.

use crate::error::{Error, Result};
use serde::Deserialize;
use std::path::Path;

/// Format version of the plain-text corpus. Independent of the serialized
/// content schema: a schema change adds fields here rather than changing this
/// number. A change to what the files mean (a renamed key, a new default, a
/// different escape) increases it, ships a migration, and is applied to a
/// corpus as one mechanical change. See `docs/corpus-format.md`.
///
/// Format 2 (#450) writes the trades of an archetype or class option as one flat `trades`
/// list; format 1 wrote four separate lists. The reader accepts both while the corpus
/// migrates (#451, #452); [`READABLE_FORMATS`] lists them.
///
/// Format 3 (#425) splits `books.toml`: only the hand-curated books are stored, one file each
/// under `books/`, and the compiler derives every other book from the `[[sources]]` pages.
pub const CORPUS_FORMAT: u32 = 3;

/// The format that writes trades as one flat `trades` list (#450): what `migrate-trades` targets.
/// It is not [`CORPUS_FORMAT`], so the trades step leaves the layout step to run after it.
pub const TRADES_FORMAT: u32 = 2;

/// The first format whose book layout is the split one (`books/<publication>.toml`). A corpus
/// below it stores one `books.toml`. The layout itself is read from the disk (see
/// `Corpus::load`): a corpus below this format may already hold the split layout while its other
/// migrations are pending, and a corpus at this format or above must hold it.
pub const SPLIT_BOOKS_FORMAT: u32 = 3;

/// The corpus formats this tool reads, oldest first. A reader of format 1 lowers the legacy
/// `replaces`, `alters`, `replaces_features` and `alters_features` lists to trades; `fmt`
/// writes each file back in the format its `corpus.toml` declares.
pub const READABLE_FORMATS: [u32; 3] = [1, 2, CORPUS_FORMAT];

/// The command that converts a corpus from an older format.
pub const MIGRATE_COMMAND: &str = "cast-iron-corpus migrate";

/// The `license_refs` almost every entry carries.
pub const DEFAULT_LICENSE_REFS: [&str; 2] = ["ogl-1.0a", "legal#community-use"];

/// What `corpus.toml` declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub format: u32,
    /// The `license_refs` of an entry that does not write the key.
    pub license_refs: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            format: CORPUS_FORMAT,
            license_refs: DEFAULT_LICENSE_REFS
                .iter()
                .map(|value| value.to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    format: u32,
    defaults: RawDefaults,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDefaults {
    license_refs: Vec<String>,
}

pub fn write_config(config: &Config) -> String {
    format!(
        "format = {}\n\n[defaults]\nlicense_refs = {}\n",
        config.format,
        crate::tomlutil::string_array(&config.license_refs)
    )
}

/// Reads `corpus.toml`. A format outside [`READABLE_FORMATS`] is refused with
/// the command that fixes it.
pub fn read_config(path: &Path, text: &str) -> Result<Config> {
    let line_of = |needle: &str| {
        text.lines()
            .position(|line| line.trim_start().starts_with(needle))
            .map_or(1, |index| index + 1)
    };
    let raw: RawConfig = toml::from_str(text).map_err(|error| {
        let line = error.span().map_or(1, |span| {
            1 + text[..span.start.min(text.len())]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
        });
        Error::new(path, line, format!("corpus.toml: {error}"))
    })?;
    if !READABLE_FORMATS.contains(&raw.format) {
        let advice = if raw.format < CORPUS_FORMAT {
            format!("run `{MIGRATE_COMMAND}` to convert the corpus")
        } else {
            format!(
                "this tool is older than the corpus; update it, or run `{MIGRATE_COMMAND}` from the newer tool to convert back"
            )
        };
        return Err(Error::new(
            path,
            line_of("format"),
            format!(
                "the corpus is format {}, and this tool reads formats {}; {advice}",
                raw.format,
                READABLE_FORMATS.map(|format| format.to_string()).join(", ")
            ),
        ));
    }
    Ok(Config {
        format: raw.format,
        license_refs: raw.defaults.license_refs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_config_round_trips() {
        let config = Config::default();
        let text = write_config(&config);
        assert_eq!(
            text,
            "format = 3\n\n[defaults]\nlicense_refs = [\"ogl-1.0a\", \"legal#community-use\"]\n"
        );
        assert_eq!(
            read_config(Path::new("corpus.toml"), &text).unwrap(),
            config
        );
    }

    #[test]
    fn an_unknown_format_is_refused_and_names_the_migration_command() {
        for (format, fragment) in [(0, "to convert the corpus"), (4, "older than the corpus")] {
            let text = format!("format = {format}\n\n[defaults]\nlicense_refs = []\n");
            let error = read_config(Path::new("corpus.toml"), &text).unwrap_err();
            assert_eq!(error.line, 1);
            assert!(error.message.contains(MIGRATE_COMMAND), "{error}");
            assert!(error.message.contains(fragment), "{error}");
        }
    }

    #[test]
    fn unknown_keys_and_missing_defaults_are_errors() {
        let path = Path::new("corpus.toml");
        assert!(
            read_config(
                path,
                "format = 1\n[defaults]\nlicense_refs = []\nextra = 1\n"
            )
            .is_err()
        );
        assert!(read_config(path, "format = 1\n").is_err());
        assert!(read_config(path, "[defaults]\nlicense_refs = []\n").is_err());
    }
}
