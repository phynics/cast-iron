//! The published-id guard: no id that a published library has shipped may stop resolving.
//!
//! `sources/published-ids.txt` lists every id that a published library has used, one per line,
//! sorted, with `#` comment lines. A bookmark can hold any of them. The build fails when one no
//! longer resolves, either directly to an entry or through an `aliases.toml` row to an entry.
//! The file only grows: see `docs/releasing.md` for how a new library adds its ids.

use cast_iron_schema::AliasIndex;
use std::collections::{BTreeMap, BTreeSet};

/// The ids in `text`, in file order. Blank lines and lines that start with `#` are skipped.
/// Returns the problems with the file (an unsorted or duplicated line) instead of ids when it
/// is malformed, so the file stays a clean sorted set that diffs show as additions.
pub fn parse(text: &str) -> Result<Vec<&str>, Vec<String>> {
    let mut ids: Vec<(usize, &str)> = Vec::new();
    let mut problems = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.trim() != line {
            problems.push(format!(
                "published-ids.txt:{line_number}: surrounding whitespace"
            ));
        }
        if let Some((_, previous)) = ids.last() {
            if *previous == line {
                problems.push(format!(
                    "published-ids.txt:{line_number}: duplicate {line:?}"
                ));
            } else if *previous > line {
                problems.push(format!(
                    "published-ids.txt:{line_number}: not sorted ({line:?} after {previous:?})"
                ));
            }
        }
        ids.push((line_number, line));
    }
    if problems.is_empty() {
        Ok(ids.into_iter().map(|(_, id)| id).collect())
    } else {
        Err(problems)
    }
}

/// One line per published id that no longer resolves. Empty when every id does.
///
/// An id resolves when it is an entry id, or when `aliases` maps it to an entry id. A retired
/// alias row does not count: a published id may not be retired without a replacement.
pub fn unresolved(
    published: &[&str],
    entry_ids: &BTreeSet<&str>,
    aliases: Option<&AliasIndex>,
) -> Vec<String> {
    let targets: BTreeMap<&str, &str> = aliases
        .map(|index| {
            index
                .entries
                .iter()
                .map(|row| (row.old.as_str(), row.target.as_str()))
                .collect()
        })
        .unwrap_or_default();
    let retired: BTreeSet<&str> = aliases
        .map(|index| index.retired.iter().map(|row| row.old.as_str()).collect())
        .unwrap_or_default();
    published
        .iter()
        .filter_map(|id| {
            if entry_ids.contains(id) {
                return None;
            }
            match targets.get(id) {
                Some(target) if entry_ids.contains(target) => None,
                Some(target) => Some(format!(
                    "published id {id} maps to {target} in aliases.toml, but that entry is gone"
                )),
                None if retired.contains(id) => Some(format!(
                    "published id {id} is retired in aliases.toml; a published id needs a target"
                )),
                None => Some(format!(
                    "published id {id} is no longer an entry and has no row in aliases.toml"
                )),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{AliasBasis, AliasEntry, EntryId, RetiredEntry, RetirementReason};

    fn ids<'a>(list: &[&'a str]) -> BTreeSet<&'a str> {
        list.iter().copied().collect()
    }

    fn index(entries: &[(&str, &str)]) -> AliasIndex {
        AliasIndex {
            entries: entries
                .iter()
                .map(|(old, target)| AliasEntry {
                    old: EntryId::parse(*old).unwrap(),
                    target: EntryId::parse(*target).unwrap(),
                    basis: AliasBasis::Direct,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let text = "# header\n\nlegacy/a\nref/rule/b\n";
        assert_eq!(parse(text).unwrap(), ["legacy/a", "ref/rule/b"]);
    }

    #[test]
    fn an_unsorted_or_duplicated_file_is_rejected_with_its_line() {
        let problems = parse("b\na\na\n").unwrap_err();
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].contains(":2:") && problems[0].contains("not sorted"));
        assert!(problems[1].contains(":3:") && problems[1].contains("duplicate"));
    }

    #[test]
    fn an_id_resolves_directly_or_through_an_alias() {
        let entries = ids(&["ref/rule/b", "ref/rule/c"]);
        let aliases = index(&[("legacy/a", "ref/rule/b")]);
        let published = ["legacy/a", "ref/rule/b", "ref/rule/c"];
        assert!(unresolved(&published, &entries, Some(&aliases)).is_empty());
    }

    #[test]
    fn a_dropped_id_is_named() {
        let entries = ids(&["ref/rule/b"]);
        let found = unresolved(&["ref/rule/gone", "ref/rule/b"], &entries, None);
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("ref/rule/gone") && found[0].contains("no row"));
    }

    #[test]
    fn an_alias_to_a_missing_entry_and_a_retired_row_do_not_resolve() {
        let entries = ids(&["ref/rule/b"]);
        let mut aliases = index(&[("legacy/a", "ref/rule/missing")]);
        aliases.retired.push(RetiredEntry {
            old: EntryId::parse("legacy/r").unwrap(),
            reason: RetirementReason::NoSource,
            note: None,
        });
        let found = unresolved(&["legacy/a", "legacy/r"], &entries, Some(&aliases));
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains("that entry is gone"));
        assert!(found[1].contains("retired"));
    }
}
