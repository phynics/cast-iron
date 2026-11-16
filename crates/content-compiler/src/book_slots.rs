//! The share-slot guard (#408): every book has a permanent slot, and the build fails when one
//! has none.
//!
//! `sources/published-books.txt` lists book ids, one per line, with `#` comment lines. A book's
//! slot is its position among the id lines, counted from 0. The build gives each catalog source
//! its slot (`SourceInfo.share_slot`) and fails when a source has no line. A line whose book is
//! no longer in the library keeps its slot; a code that names it is skipped on the receiver,
//! which is what a removed book needs.
//!
//! **Permanence is enforced outside this crate, against git history.** A share code
//! (`crates/web/src/book_code.rs`) names slots, so an edited, moved or deleted line would change
//! what every code already shared means. The compiler cannot see that: it trusts the file it is
//! given. `cargo xtask check-book-slots` compares the file with its version on the base branch
//! and fails unless every id line the base has is at the same position (only appends pass). CI
//! runs it on every pull request with the base commit; `cargo xtask check` runs it against
//! `origin/main`. What it does not cover is a direct push to the base branch, which skips review.
//!
//! Unlike `sources/published-ids.txt`, the file is not sorted: the order is the assignment.

use cast_iron_schema::{MAX_SHARE_SLOTS, SourceInfo};
use std::collections::BTreeMap;

/// The ids in `text`, in slot order. Blank lines and `#` lines are skipped. Returns the problems
/// with the file (surrounding whitespace, a duplicate, too many lines) instead when it is malformed.
pub fn parse(text: &str) -> Result<Vec<&str>, Vec<String>> {
    let mut ids: Vec<&str> = Vec::new();
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut problems = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.trim() != line || line.contains(char::is_whitespace) {
            problems.push(format!(
                "published-books.txt:{line_number}: a book id has no whitespace ({line:?})"
            ));
        }
        if let Some(first) = seen.insert(line, line_number) {
            problems.push(format!(
                "published-books.txt:{line_number}: duplicate {line:?} (first on line {first})"
            ));
            continue;
        }
        ids.push(line);
    }
    if ids.len() > MAX_SHARE_SLOTS as usize {
        problems.push(format!(
            "published-books.txt lists {} books; a share code holds at most {MAX_SHARE_SLOTS}",
            ids.len()
        ));
    }
    if problems.is_empty() {
        Ok(ids)
    } else {
        Err(problems)
    }
}

/// Gives each source its slot. Returns one line per source that has no line in the file.
pub fn assign(listed: &[&str], sources: &mut [SourceInfo]) -> Result<(), Vec<String>> {
    let slots: BTreeMap<&str, u32> = listed
        .iter()
        .enumerate()
        .map(|(slot, id)| (*id, slot as u32))
        .collect();
    let mut missing = Vec::new();
    for source in sources.iter_mut() {
        match slots.get(source.id.as_str()) {
            Some(slot) => source.share_slot = Some(*slot),
            None => missing.push(source.id.clone()),
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    missing.sort();
    let mut lines = vec![format!(
        "{} book(s) have no slot in sources/published-books.txt; run `cargo xtask published-books` and commit the added lines:",
        missing.len()
    )];
    lines.extend(missing.iter().take(50).map(|id| format!("  {id}")));
    if missing.len() > 50 {
        lines.push(format!("  ... and {} more", missing.len() - 50));
    }
    Err(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str) -> SourceInfo {
        SourceInfo {
            id: id.to_string(),
            title: id.to_string(),
            entry_count: 0,
            copyright: None,
            category: None,
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
            aon: None,
            share_slot: None,
        }
    }

    #[test]
    fn the_line_order_is_the_slot_order() {
        let listed = parse("# header\n\nzeta\nalpha\nmid\n").unwrap();
        assert_eq!(listed, ["zeta", "alpha", "mid"]);
        let mut sources = [source("alpha"), source("mid"), source("zeta")];
        assign(&listed, &mut sources).unwrap();
        let slots: Vec<_> = sources.iter().map(|s| s.share_slot).collect();
        assert_eq!(slots, [Some(1), Some(2), Some(0)]);
    }

    #[test]
    fn a_book_that_left_the_library_keeps_its_line() {
        let listed = parse("gone\nkept\n").unwrap();
        let mut sources = [source("kept")];
        assign(&listed, &mut sources).unwrap();
        assert_eq!(sources[0].share_slot, Some(1));
    }

    #[test]
    fn a_new_book_without_a_line_fails_the_build_and_names_it() {
        let listed = parse("a\n").unwrap();
        let mut sources = [source("a"), source("new-book")];
        let lines = assign(&listed, &mut sources).unwrap_err();
        assert!(
            lines[0].contains("cargo xtask published-books"),
            "{lines:?}"
        );
        assert!(lines.iter().any(|line| line.trim() == "new-book"));
    }

    #[test]
    fn a_duplicate_or_padded_line_is_rejected_with_its_line_number() {
        let problems = parse("a\nb\na\n b\n").unwrap_err();
        assert!(
            problems
                .iter()
                .any(|p| p.contains(":3:") && p.contains("duplicate"))
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains(":4:") && p.contains("whitespace"))
        );
    }

    #[test]
    fn the_file_may_not_outgrow_the_code() {
        let text: String = (0..=MAX_SHARE_SLOTS).map(|i| format!("b{i}\n")).collect();
        assert!(parse(&text).is_err());
    }
}
