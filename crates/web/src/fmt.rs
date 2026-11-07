//! Display helpers that do not need the browser.

/// `11735022` becomes `11.2 MB`. Uses 1024-based units.
pub fn bytes(value: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else if size >= 100.0 {
        format!("{size:.0} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

/// Groups the digits of a count with commas: `3583` becomes `3,583`. The counts on Home, in
/// category and filter lists, in search, on entry pages, in Settings and on the legal page go
/// through here, so those screens write numbers one way. There is no locale setting, so the separator is always a comma.
pub fn count(n: impl TryInto<u64>) -> String {
    let digits = n
        .try_into()
        .map_or_else(|_| String::new(), |n| n.to_string());
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// `2009-08-01` becomes `Aug 2009`; a date with a year only becomes `2009`. The month names are
/// a fixed English table: the app has no locale setting, and `Intl` would read the device's.
/// `None` for anything that does not start with a four-digit year.
pub fn month_year(date: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut parts = date.trim().split('-');
    let year = parts.next()?;
    if year.len() != 4 || !year.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let month = parts
        .next()
        .and_then(|m| m.parse::<usize>().ok())
        .filter(|m| (1..=12).contains(m));
    Some(match month {
        Some(m) => format!("{} {year}", MONTHS[m - 1]),
        None => year.to_string(),
    })
}

/// The first 8 characters of a hash, with an ellipsis when it is longer. Settings shows this
/// shortened copy in parentheses so the full hash does not dominate the line.
pub fn short_hash(value: &str) -> String {
    match value.get(..8) {
        Some(short) if short.len() < value.len() => format!("{short}…"),
        _ => value.to_string(),
    }
}

/// `2026-11-07` becomes `7 Nov 2026`. The month names are a fixed English table, like
/// [`month_year`], so a device's locale never changes an app date.
pub fn day_month_year(date: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut parts = date.trim().split('-');
    let year = parts.next()?;
    if year.len() != 4 || !year.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let month = parts
        .next()?
        .parse::<usize>()
        .ok()
        .filter(|month| (1..=12).contains(month))?;
    let day = parts
        .next()?
        .parse::<u32>()
        .ok()
        .filter(|day| (1..=31).contains(day))?;
    if parts.next().is_some() {
        return None;
    }
    Some(format!("{day} {} {year}", MONTHS[month - 1]))
}

/// The first 8 hex characters of a content version, keeping its `v` prefix:
/// `v48ce09016271a01a` becomes `v48ce0901…`.
fn short_content_version(value: &str) -> String {
    let (prefix, body) = match value.strip_prefix('v') {
        Some(body) => ("v", body),
        None => ("", value),
    };
    match body.char_indices().nth(8) {
        Some((at, _)) => format!("{prefix}{}…", &body[..at]),
        None => value.to_string(),
    }
}

/// Settings: `Corpus v0.1.6 (v48ce0901…)`. A library without a tag shows its hash alone.
pub fn corpus_label(tag: Option<&str>, content_version: &str) -> String {
    match tag {
        Some(tag) => format!("{tag} ({})", short_content_version(content_version)),
        None => content_version.to_string(),
    }
}

/// Settings: `Released 7 Nov 2026 (5ec392ee…)`. A build without a commit date shows the shell
/// hash alone.
pub fn app_label(date: Option<&str>, shell_version: &str) -> String {
    match date.and_then(day_month_year) {
        Some(date) => format!("Released {date} ({})", short_hash(shell_version)),
        None => shell_version.to_string(),
    }
}

/// The line under a book's title and on its page: `Aug 2009 · 2,994 entries`, or the count alone
/// when the book has no usable date.
pub fn book_meta(date: Option<&str>, entries: usize) -> String {
    let noun = if entries == 1 { "entry" } else { "entries" };
    let count = format!("{} {noun}", count(entries));
    match date.and_then(month_year) {
        Some(when) => format!("{when} · {count}"),
        None => count,
    }
}

/// A copyright text as the separate notices it holds, each printed once.
///
/// The corpus joins the Section 15 notices of a publication with `; `, and the import repeated
/// a notice when the source files carried it with another title in front or `Copyright` for
/// `©`. A repeat is a notice whose text from the copyright sign on is the same once case, white
/// space, the word `Copyright` and trailing punctuation are ignored. Notices that differ in any
/// other way (a year, an author list) are all kept: a reprint's notice is a different notice.
/// The first position wins and the longer spelling is kept, so a title in front survives.
pub fn notice_lines(copyright: &str) -> Vec<String> {
    let mut notices: Vec<String> = Vec::new();
    for piece in copyright.split("; ") {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        // `Author:` continues the notice before it: `© 2010, Paizo, LLC; Author: Jason Bulmahn.`
        match notices.last_mut() {
            Some(last) if piece.starts_with("Author") => {
                last.push_str("; ");
                last.push_str(piece);
            }
            _ => notices.push(piece.to_string()),
        }
    }
    let mut kept: Vec<(String, String)> = Vec::new();
    for notice in notices {
        let key = notice_key(&notice);
        match kept.iter_mut().find(|(known, _)| *known == key) {
            Some((_, text)) => {
                if notice.len() > text.len() {
                    *text = notice;
                }
            }
            None => kept.push((key, notice)),
        }
    }
    kept.into_iter().map(|(_, text)| text).collect()
}

fn notice_key(notice: &str) -> String {
    let start = ["©", "Copyright", "copyright"]
        .iter()
        .filter_map(|mark| notice.find(mark))
        .min()
        .unwrap_or(0);
    let tail = notice[start..]
        .replace("Copyright", "©")
        .replace("copyright", "©");
    let mut key = String::with_capacity(tail.len());
    for word in tail.split_whitespace() {
        if !key.is_empty() {
            key.push(' ');
        }
        key.push_str(word);
    }
    // `© 2010, Paizo` and `© 2010 Paizo` are one notice.
    key.to_lowercase()
        .replace(',', "")
        .trim_end_matches(['.', ';', ' '])
        .to_string()
}

/// Writes a count with its noun: `0 entries`, `1 entry`, `2 entries`. The number goes through
/// [`count`], so every counted label groups digits the same way.
pub fn pluralize(n: impl TryInto<u64>, singular: &str, plural: &str) -> String {
    let n = n.try_into().unwrap_or_default();
    format!("{} {}", count(n), if n == 1 { singular } else { plural })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_and_year_use_a_fixed_english_table() {
        assert_eq!(month_year("2009-08-01").as_deref(), Some("Aug 2009"));
        assert_eq!(month_year("2010-01").as_deref(), Some("Jan 2010"));
        assert_eq!(month_year("2012-12-31").as_deref(), Some("Dec 2012"));
        assert_eq!(month_year("2011").as_deref(), Some("2011"));
        // A month outside 1 to 12 is dropped, not guessed.
        assert_eq!(month_year("2011-13-01").as_deref(), Some("2011"));
        assert_eq!(month_year("2011-00-01").as_deref(), Some("2011"));
        for bad in ["", "Aug 2009", "209-08", "20x9-08", "-"] {
            assert_eq!(month_year(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn settings_labels_keep_the_hash_in_parentheses() {
        assert_eq!(
            corpus_label(Some("v0.1.6"), "v48ce09016271a01a"),
            "v0.1.6 (v48ce0901…)"
        );
        assert_eq!(corpus_label(None, "v48ce09016271a01a"), "v48ce09016271a01a");
        assert_eq!(
            app_label(Some("2026-11-07"), "5ec392ee12345678"),
            "Released 7 Nov 2026 (5ec392ee…)"
        );
        assert_eq!(app_label(None, "5ec392ee12345678"), "5ec392ee12345678");
        assert_eq!(
            app_label(Some("not-a-date"), "5ec392ee12345678"),
            "5ec392ee12345678"
        );
    }

    #[test]
    fn day_month_year_uses_the_fixed_english_table() {
        assert_eq!(day_month_year("2026-11-07").as_deref(), Some("7 Nov 2026"));
        assert_eq!(day_month_year("2026-01-31").as_deref(), Some("31 Jan 2026"));
        for bad in [
            "",
            "2026-11",
            "2026-13-01",
            "2026-11-00",
            "2026-11-32",
            "2026-11-07T00:00",
            "26-11-07",
        ] {
            assert_eq!(day_month_year(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_book_meta_line_joins_the_date_and_the_grouped_count() {
        assert_eq!(
            book_meta(Some("2009-08-01"), 2994),
            "Aug 2009 · 2,994 entries"
        );
        assert_eq!(book_meta(Some("2009-08-01"), 1), "Aug 2009 · 1 entry");
        assert_eq!(book_meta(None, 12), "12 entries");
        assert_eq!(book_meta(Some("unknown"), 0), "0 entries");
    }

    #[test]
    fn a_repeated_notice_is_printed_once() {
        // The Advanced Player's Guide: one notice, three times, once with the title in front.
        let apg = "Advanced Player’s Guide, © 2010, Paizo Publishing, LLC; Author: Jason Bulmahn.; \
                   © 2010, Paizo Publishing, LLC; Author: Jason Bulmahn.; \
                   © 2010, Paizo Publishing, LLC; Author: Jason Bulmahn";
        assert_eq!(
            notice_lines(apg),
            ["Advanced Player’s Guide, © 2010, Paizo Publishing, LLC; Author: Jason Bulmahn."]
        );
        // `Copyright` for `©` and a different title in front: the same notice.
        let armory = "© 2010, Paizo Publishing, LLC; Authors: Jonathan Keith, Hal Maclean.; \
                      Pathfinder Companion: Adventurer’s Armory. Copyright 2010, Paizo Publishing, LLC; \
                      Authors: Jonathan Keith, Hal Maclean.";
        assert_eq!(
            notice_lines(armory),
            [
                "Pathfinder Companion: Adventurer’s Armory. Copyright 2010, Paizo Publishing, LLC; Authors: Jonathan Keith, Hal Maclean."
            ]
        );
    }

    #[test]
    fn notices_that_differ_are_all_kept() {
        // A reprint has its own year: both notices are required, in their order.
        let core = "Pathfinder RPG Core Rulebook. Copyright 2009, Paizo Publishing, LLC; \
                    Author: Jason Bulmahn.; © 2010, Paizo Publishing, LLC; Author: Jason Bulmahn.";
        assert_eq!(notice_lines(core).len(), 2);
        // Another author list is another notice, even for the same year.
        let two = "© 2011, Paizo Publishing, LLC; Authors: A and B.; © 2011, Paizo Publishing, LLC; Authors: A and C.";
        assert_eq!(notice_lines(two).len(), 2);
        assert_eq!(notice_lines("One notice."), ["One notice."]);
        assert!(notice_lines("").is_empty());
        assert!(notice_lines(" ; ").is_empty());
    }

    #[test]
    fn groups_digits_by_three() {
        assert_eq!(count(0usize), "0");
        assert_eq!(count(7usize), "7");
        assert_eq!(count(999usize), "999");
        assert_eq!(count(1000usize), "1,000");
        assert_eq!(count(3583usize), "3,583");
        assert_eq!(count(29_412u32), "29,412");
        assert_eq!(count(1_234_567u64), "1,234,567");
    }

    #[test]
    fn pluralize_picks_the_singular_only_for_one() {
        assert_eq!(pluralize(0, "entry", "entries"), "0 entries");
        assert_eq!(pluralize(1, "entry", "entries"), "1 entry");
        assert_eq!(pluralize(2, "entry", "entries"), "2 entries");
        assert_eq!(
            pluralize(1324, "publication entry", "publication entries"),
            "1,324 publication entries"
        );
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(1023), "1023 B");
        assert_eq!(bytes(1536), "1.5 KB");
        assert_eq!(bytes(11_735_022), "11.2 MB");
        assert_eq!(bytes(300 * 1024 * 1024), "300 MB");
    }
}
