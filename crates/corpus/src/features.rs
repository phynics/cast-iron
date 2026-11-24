//! The `features` of an archetype, class option or base class (corpus format 4, #540).
//!
//! ```toml
//! features = [
//!   { anchor = "shattering-strike", level = 2 },
//! ]
//! ```
//!
//! `anchor` is the key of a heading of the body (`## Shattering Strike (Ex) {#shattering-strike}`)
//! and `level` is the level at which the feature is gained. The list holds facts that the printed
//! text states in prose, so that the app can show a head and a jump without rewriting the prose
//! into rows. A trade names the feature that states it with `by` (see `trades`).

use crate::error::{Error, Result};
use crate::tomlutil;
use cast_iron_schema::FeatureFact;
use std::path::Path;

/// The first corpus format that may write `features` and the `by` key of a trade.
pub const FEATURES_FORMAT: u32 = 4;

/// The kinds of typed fields that may carry `features`.
pub fn kind_has_features(kind: &str) -> bool {
    matches!(kind, "archetype" | "class_option" | "class")
}

/// Reads `features` from a `[fields]` table. A corpus below [`FEATURES_FORMAT`] has none; the
/// key itself is refused by the allowed-key check of `entry`.
pub fn read_features(path: &Path, table: &toml::Table, line: usize) -> Result<Vec<FeatureFact>> {
    let Some(value) = table.get("features") else {
        return Ok(Vec::new());
    };
    let toml::Value::Array(rows) = value else {
        return Err(Error::new(
            path,
            line,
            "`features` must be an array of tables, one per feature",
        ));
    };
    let mut out = Vec::new();
    for row in rows {
        let row = row
            .as_table()
            .ok_or_else(|| Error::new(path, line, "`features` must hold tables"))?;
        for key in row.keys() {
            if key != "anchor" && key != "level" {
                return Err(Error::new(
                    path,
                    line,
                    format!(
                        "unexpected key `{key}` in a feature; a feature has `anchor` and `level`"
                    ),
                ));
            }
        }
        let anchor = match row.get("anchor") {
            Some(toml::Value::String(anchor)) if !anchor.trim().is_empty() => anchor.clone(),
            _ => {
                return Err(Error::new(
                    path,
                    line,
                    "a feature needs `anchor`, the key of a heading of the body",
                ));
            }
        };
        let level = row
            .get("level")
            .and_then(toml::Value::as_integer)
            .and_then(|level| u8::try_from(level).ok())
            .filter(|level| *level >= 1)
            .ok_or_else(|| {
                Error::new(
                    path,
                    line,
                    format!("the feature `{anchor}` needs `level`, a whole number from 1 to 255"),
                )
            })?;
        out.push(FeatureFact { anchor, level });
    }
    Ok(out)
}

/// Writes `features`, one inline table per feature, in the order they are held.
pub fn write_features(out: &mut String, features: &[FeatureFact]) {
    if features.is_empty() {
        return;
    }
    out.push_str("features = [\n");
    for feature in features {
        out.push_str("  { anchor = ");
        out.push_str(&tomlutil::string(&feature.anchor));
        out.push_str(", level = ");
        out.push_str(&feature.level.to_string());
        out.push_str(" },\n");
    }
    out.push_str("]\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> toml::Table {
        toml::from_str(text).unwrap()
    }

    #[test]
    fn features_round_trip() {
        let features = vec![
            FeatureFact {
                anchor: "shattering-strike".into(),
                level: 2,
            },
            FeatureFact {
                anchor: "overhand-chop".into(),
                level: 3,
            },
        ];
        let mut out = String::new();
        write_features(&mut out, &features);
        assert_eq!(
            out,
            "features = [\n  { anchor = \"shattering-strike\", level = 2 },\n  { anchor = \"overhand-chop\", level = 3 },\n]\n"
        );
        let back = read_features(Path::new("x.md"), &table(&out), 1).unwrap();
        assert_eq!(back, features);
        let mut empty = String::new();
        write_features(&mut empty, &[]);
        assert!(empty.is_empty());
    }

    #[test]
    fn a_feature_needs_an_anchor_and_a_level_and_nothing_else() {
        let read = |text: &str| read_features(Path::new("x.md"), &table(text), 1);
        assert!(read("features = [{ level = 2 }]").is_err());
        assert!(read("features = [{ anchor = \"a\" }]").is_err());
        assert!(read("features = [{ anchor = \"a\", level = 0 }]").is_err());
        assert!(read("features = [{ anchor = \"a\", level = 300 }]").is_err());
        let error = read("features = [{ anchor = \"a\", level = 1, name = \"A\" }]").unwrap_err();
        assert!(error.message.contains("unexpected key `name`"), "{error:?}");
        assert!(read("features = \"a\"").is_err());
        assert!(read("").unwrap().is_empty());
    }
}
