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
/// through here, so those screens write numbers one way. The Books directory, a book's contents
/// (`toc-count`) and the book count line (`book_count_line`) still print plain numbers: the Books
/// rows change (#154) owns them. There is no locale setting, so the separator is always a comma.
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
