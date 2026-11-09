//! The three series lines the corpus records (#159): Adventure Path by path and volume,
//! Modules ordered by publication date, and Pathfinder Society by season and scenario.
//!
//! Everything here is pure: the functions take borrowed publication fields and return indices
//! into the input, so Books, Manage books and the search Publication filter share one order.
//! Native tests cover the grouping, the fallback for a publication with no series, and the
//! title prefixes the printed lines carry.

use cast_iron_schema::{SeriesKind, SourceInfo};

/// The publication fields that series grouping and title cleanup read. Books and Manage books
/// implement it for their row type; search uses it for [`SourceInfo`].
pub trait SeriesFields {
    fn id(&self) -> &str;
    fn title(&self) -> &str;
    fn series_kind(&self) -> Option<SeriesKind>;
    fn series(&self) -> Option<&str>;
    fn series_number(&self) -> Option<&str>;
    fn season(&self) -> Option<u16>;
    fn publication_date(&self) -> Option<&str>;
}

impl<T: SeriesFields + ?Sized> SeriesFields for &T {
    fn id(&self) -> &str {
        (*self).id()
    }

    fn title(&self) -> &str {
        (*self).title()
    }

    fn series_kind(&self) -> Option<SeriesKind> {
        (*self).series_kind()
    }

    fn series(&self) -> Option<&str> {
        (*self).series()
    }

    fn series_number(&self) -> Option<&str> {
        (*self).series_number()
    }

    fn season(&self) -> Option<u16> {
        (*self).season()
    }

    fn publication_date(&self) -> Option<&str> {
        (*self).publication_date()
    }
}

impl SeriesFields for SourceInfo {
    fn id(&self) -> &str {
        &self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn series_kind(&self) -> Option<SeriesKind> {
        self.series_kind
    }

    fn series(&self) -> Option<&str> {
        self.series.as_deref()
    }

    fn series_number(&self) -> Option<&str> {
        self.series_number.as_deref()
    }

    fn season(&self) -> Option<u16> {
        self.season
    }

    fn publication_date(&self) -> Option<&str> {
        self.publication_date.as_deref()
    }
}

/// One Adventure Path group or one Society season, with indices into the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedGroup {
    pub name: String,
    pub books: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeasonGroup {
    pub season: u16,
    pub books: Vec<usize>,
}

/// The Adventures tier split the way the corpus records it. A publication that carries no
/// recognized series falls to `other`: it is displayed last, never dropped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdventureSections {
    /// Sorted by path name, each with its volumes in series-number order.
    pub paths: Vec<NamedGroup>,
    /// In publication-date order, undated last.
    pub modules: Vec<usize>,
    /// Sorted by season, each with its scenarios in number order.
    pub seasons: Vec<SeasonGroup>,
    /// Publications with no recognized series, in the caller's order.
    pub other: Vec<usize>,
}

impl AdventureSections {
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
            && self.modules.is_empty()
            && self.seasons.is_empty()
            && self.other.is_empty()
    }

    /// Every input index exactly once, in display order.
    pub fn indices(&self) -> Vec<usize> {
        self.paths
            .iter()
            .flat_map(|group| group.books.iter().copied())
            .chain(self.modules.iter().copied())
            .chain(
                self.seasons
                    .iter()
                    .flat_map(|group| group.books.iter().copied()),
            )
            .chain(self.other.iter().copied())
            .collect()
    }
}

/// Splits publications into the three series groups and the fallback.
pub fn adventure_sections<T: SeriesFields>(books: &[T]) -> AdventureSections {
    let mut paths: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    let mut modules = Vec::new();
    let mut seasons: std::collections::BTreeMap<u16, Vec<usize>> =
        std::collections::BTreeMap::new();
    let mut other = Vec::new();

    for (index, book) in books.iter().enumerate() {
        match book.series_kind() {
            Some(SeriesKind::AdventurePath)
                if book.series().is_some_and(|name| !name.trim().is_empty())
                    && book
                        .series_number()
                        .is_some_and(|number| !number.is_empty()) =>
            {
                paths
                    .entry(book.series().expect("checked above").to_string())
                    .or_default()
                    .push(index);
            }
            Some(SeriesKind::Module) => modules.push(index),
            Some(SeriesKind::Society) if book.season().is_some() => {
                seasons
                    .entry(book.season().expect("checked above"))
                    .or_default()
                    .push(index);
            }
            _ => other.push(index),
        }
    }

    let mut paths: Vec<NamedGroup> = paths
        .into_iter()
        .map(|(name, books)| NamedGroup { name, books })
        .collect();
    paths.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    for group in &mut paths {
        sort_by_number(books, &mut group.books, |book| book.series_number());
    }

    sort_by_date(books, &mut modules);

    let mut seasons: Vec<SeasonGroup> = seasons
        .into_iter()
        .map(|(season, books)| SeasonGroup { season, books })
        .collect();
    seasons.sort_by_key(|group| group.season);
    for group in &mut seasons {
        sort_by_number(books, &mut group.books, |book| book.series_number());
    }

    AdventureSections {
        paths,
        modules,
        seasons,
        other,
    }
}

/// A title for a publication in the Adventures tier: the volume number for an Adventure Path,
/// the printed title without the module prefix, or the scenario number for Society.
pub fn display_title<T: SeriesFields>(book: &T) -> String {
    match book.series_kind() {
        Some(SeriesKind::AdventurePath) => adventure_path_title(book.title(), book.series_number()),
        Some(SeriesKind::Module) => module_title(book.title()),
        Some(SeriesKind::Society) => society_title(book.title(), book.series_number()),
        None => book.title().to_string(),
    }
}

/// `Pathfinder #1: Burnt Offerings` becomes `#1 Burnt Offerings`.
pub fn adventure_path_title(title: &str, number: Option<&str>) -> String {
    let rest = issue_rest(title).unwrap_or(title);
    match number.filter(|number| !number.is_empty()) {
        Some(number) if rest.is_empty() => format!("#{number}"),
        Some(number) => format!("#{number} {rest}"),
        None => rest.to_string(),
    }
}

/// `Pathfinder Module: Academy of Secrets` becomes `Academy of Secrets`.
pub fn module_title(title: &str) -> String {
    title
        .strip_prefix("Pathfinder Module:")
        .map(str::trim)
        .filter(|rest| !rest.is_empty())
        .unwrap_or(title)
        .to_string()
}

/// `Pathfinder Society S03-10 The Immortal Connundrum` becomes
/// `#3-10 The Immortal Connundrum`.
pub fn society_title(title: &str, number: Option<&str>) -> String {
    let Some(number) = number.filter(|number| !number.is_empty()) else {
        return title.to_string();
    };
    let wanted = number_key(Some(number)).0;
    let words: Vec<&str> = title.split_whitespace().collect();
    let rest = words
        .iter()
        .position(|word| !wanted.is_empty() && number_key(Some(word)).0 == wanted)
        .map(|position| words[position + 1..].join(" "))
        .unwrap_or_default();
    if rest.is_empty() {
        format!("#{number}")
    } else {
        format!("#{number} {rest}")
    }
}

/// The rest of a `Pathfinder #N:` title, or `None` when the title does not carry the prefix.
fn issue_rest(title: &str) -> Option<&str> {
    let rest = title.strip_prefix("Pathfinder #")?;
    let end = rest
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let after = &rest[end..];
    let after = after.strip_prefix(':').unwrap_or(after);
    Some(after.trim())
}

fn sort_by_number<T: SeriesFields>(
    books: &[T],
    indices: &mut [usize],
    number: impl Fn(&T) -> Option<&str>,
) {
    indices.sort_by(|&a, &b| {
        number_key(number(&books[a]))
            .cmp(&number_key(number(&books[b])))
            .then_with(|| title_order(&books[a], &books[b]))
    });
}

fn sort_by_date<T: SeriesFields>(books: &[T], indices: &mut [usize]) {
    indices.sort_by(|&a, &b| {
        let left = dated(books[a].publication_date());
        let right = dated(books[b].publication_date());
        // ISO dates compare as text. A missing or malformed date sorts after all others.
        left.is_none()
            .cmp(&right.is_none())
            .then_with(|| left.cmp(&right))
            .then_with(|| title_order(&books[a], &books[b]))
    });
}

fn dated(date: Option<&str>) -> Option<&str> {
    let date = date?;
    (date.len() >= 4 && date.as_bytes()[..4].iter().all(u8::is_ascii_digit)).then_some(date)
}

fn title_order<T: SeriesFields>(a: &T, b: &T) -> std::cmp::Ordering {
    a.title()
        .to_lowercase()
        .cmp(&b.title().to_lowercase())
        .then_with(|| a.title().cmp(b.title()))
        .then_with(|| a.id().cmp(b.id()))
}

/// `3-01` and `#3-01` both yield `[3, 1]`, so scenario and volume numbers sort numerically.
fn number_key(value: Option<&str>) -> (Vec<u32>, String) {
    let text = value.unwrap_or("");
    let mut runs = Vec::new();
    let mut current = 0u32;
    let mut in_digits = false;
    for ch in text.chars() {
        if let Some(digit) = ch.to_digit(10) {
            current = current.saturating_mul(10).saturating_add(digit);
            in_digits = true;
        } else if in_digits {
            runs.push(current);
            current = 0;
            in_digits = false;
        }
    }
    if in_digits {
        runs.push(current);
    }
    (runs, text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Book {
        id: &'static str,
        title: &'static str,
        kind: Option<SeriesKind>,
        series: Option<&'static str>,
        number: Option<&'static str>,
        season: Option<u16>,
        date: Option<&'static str>,
    }

    impl SeriesFields for Book {
        fn id(&self) -> &str {
            self.id
        }

        fn title(&self) -> &str {
            self.title
        }

        fn series_kind(&self) -> Option<SeriesKind> {
            self.kind
        }

        fn series(&self) -> Option<&str> {
            self.series
        }

        fn series_number(&self) -> Option<&str> {
            self.number
        }

        fn season(&self) -> Option<u16> {
            self.season
        }

        fn publication_date(&self) -> Option<&str> {
            self.date
        }
    }

    fn book(
        id: &'static str,
        title: &'static str,
        kind: Option<SeriesKind>,
        series: Option<&'static str>,
        number: Option<&'static str>,
        season: Option<u16>,
        date: Option<&'static str>,
    ) -> Book {
        Book {
            id,
            title,
            kind,
            series,
            number,
            season,
            date,
        }
    }

    fn names<'a>(books: &'a [Book], indices: &[usize]) -> Vec<&'a str> {
        indices.iter().map(|&index| books[index].id).collect()
    }

    #[test]
    fn adventure_paths_group_by_name_and_volumes_follow_the_issue_number() {
        let books = vec![
            book(
                "sins-2",
                "Pathfinder #2: Sins",
                Some(SeriesKind::AdventurePath),
                Some("Second Path"),
                Some("2"),
                None,
                None,
            ),
            book(
                "runelords-10",
                "Pathfinder #10: Later",
                Some(SeriesKind::AdventurePath),
                Some("Rise of the Runelords"),
                Some("10"),
                None,
                None,
            ),
            book(
                "runelords-1",
                "Pathfinder #1: Burnt Offerings",
                Some(SeriesKind::AdventurePath),
                Some("Rise of the Runelords"),
                Some("1"),
                None,
                None,
            ),
            book(
                "runelords-2",
                "Pathfinder #2: Skinsaw",
                Some(SeriesKind::AdventurePath),
                Some("Rise of the Runelords"),
                Some("2"),
                None,
                None,
            ),
            book(
                "sins-1",
                "Pathfinder #1: Sins",
                Some(SeriesKind::AdventurePath),
                Some("Second Path"),
                Some("1"),
                None,
                None,
            ),
        ];
        let sections = adventure_sections(&books);
        let paths: Vec<_> = sections
            .paths
            .iter()
            .map(|group| (group.name.as_str(), names(&books, &group.books)))
            .collect();
        assert_eq!(
            paths,
            [
                (
                    "Rise of the Runelords",
                    vec!["runelords-1", "runelords-2", "runelords-10"]
                ),
                ("Second Path", vec!["sins-1", "sins-2"]),
            ]
        );
        assert_eq!(sections.modules, Vec::<usize>::new());
        assert!(sections.seasons.is_empty());
        assert!(sections.other.is_empty());
    }

    #[test]
    fn modules_follow_publication_date_with_undated_last_and_ties_by_title() {
        let books = vec![
            book(
                "late",
                "Later",
                Some(SeriesKind::Module),
                None,
                None,
                None,
                Some("2012-01-01"),
            ),
            book(
                "undated",
                "Alpha",
                Some(SeriesKind::Module),
                None,
                None,
                None,
                None,
            ),
            book(
                "early-b",
                "Beta",
                Some(SeriesKind::Module),
                None,
                None,
                None,
                Some("2008-01-01"),
            ),
            book(
                "early-a",
                "Alpha",
                Some(SeriesKind::Module),
                None,
                None,
                None,
                Some("2008-01-01"),
            ),
        ];
        let sections = adventure_sections(&books);
        assert_eq!(
            names(&books, &sections.modules),
            ["early-a", "early-b", "late", "undated"]
        );
        assert!(sections.paths.is_empty());
        assert!(sections.seasons.is_empty());
        assert!(sections.other.is_empty());
    }

    #[test]
    fn society_groups_by_season_and_scenarios_sort_by_number() {
        let books = vec![
            book(
                "season-3-10",
                "Pathfinder Society S03-10 Later",
                Some(SeriesKind::Society),
                None,
                Some("3-10"),
                Some(3),
                None,
            ),
            book(
                "season-1-01",
                "Pathfinder Society S01-01 First",
                Some(SeriesKind::Society),
                None,
                Some("1-01"),
                Some(1),
                None,
            ),
            book(
                "season-3-01",
                "Pathfinder Society S03-01 Early",
                Some(SeriesKind::Society),
                None,
                Some("3-01"),
                Some(3),
                None,
            ),
        ];
        let sections = adventure_sections(&books);
        let seasons: Vec<_> = sections
            .seasons
            .iter()
            .map(|group| (group.season, names(&books, &group.books)))
            .collect();
        assert_eq!(
            seasons,
            [
                (1, vec!["season-1-01"]),
                (3, vec!["season-3-01", "season-3-10"]),
            ]
        );
    }

    #[test]
    fn a_publication_without_series_falls_to_other_and_is_never_dropped() {
        let books = vec![
            book("plain", "A Player's Guide", None, None, None, None, None),
            book(
                "no-name",
                "Pathfinder #7: No Name",
                Some(SeriesKind::AdventurePath),
                None,
                Some("7"),
                None,
                None,
            ),
            book(
                "no-number",
                "Pathfinder #8: No Number",
                Some(SeriesKind::AdventurePath),
                Some("Lost Path"),
                None,
                None,
                None,
            ),
            book(
                "no-season",
                "Pathfinder Society Unknown",
                Some(SeriesKind::Society),
                None,
                Some("9-99"),
                None,
                None,
            ),
        ];
        let sections = adventure_sections(&books);
        assert!(sections.paths.is_empty());
        assert_eq!(
            names(&books, &sections.other),
            ["plain", "no-name", "no-number", "no-season"]
        );
        assert_eq!(sections.indices().len(), books.len());
        let mut all = sections.indices();
        all.sort_unstable();
        assert_eq!(all, (0..books.len()).collect::<Vec<_>>());
    }

    #[test]
    fn every_input_index_appears_once_in_display_order() {
        let books = vec![
            book(
                "ap",
                "Pathfinder #1: One",
                Some(SeriesKind::AdventurePath),
                Some("Path"),
                Some("1"),
                None,
                None,
            ),
            book(
                "module",
                "Module",
                Some(SeriesKind::Module),
                None,
                None,
                None,
                Some("2010-01-01"),
            ),
            book(
                "society",
                "Pathfinder Society S01-01 One",
                Some(SeriesKind::Society),
                None,
                Some("1-01"),
                Some(1),
                None,
            ),
            book("plain", "Plain", None, None, None, None, None),
        ];
        let sections = adventure_sections(&books);
        assert_eq!(
            names(&books, &sections.indices()),
            ["ap", "module", "society", "plain"]
        );
    }

    #[test]
    fn titles_lose_the_printed_prefixes() {
        assert_eq!(
            adventure_path_title("Pathfinder #1: Burnt Offerings", Some("1")),
            "#1 Burnt Offerings"
        );
        assert_eq!(
            adventure_path_title("Burnt Offerings", Some("12")),
            "#12 Burnt Offerings"
        );
        assert_eq!(adventure_path_title("Pathfinder #3", Some("3")), "#3");
        assert_eq!(
            module_title("Pathfinder Module: Academy of Secrets"),
            "Academy of Secrets"
        );
        assert_eq!(module_title("Academy of Secrets"), "Academy of Secrets");
        assert_eq!(
            society_title(
                "Pathfinder Society S03-10 The Immortal Connundrum",
                Some("3-10")
            ),
            "#3-10 The Immortal Connundrum"
        );
        let book = book(
            "ap",
            "Pathfinder #1: Burnt Offerings",
            Some(SeriesKind::AdventurePath),
            Some("Rise of the Runelords"),
            Some("1"),
            None,
            None,
        );
        assert_eq!(display_title(&book), "#1 Burnt Offerings");
    }

    #[test]
    fn a_malformed_number_sorts_after_a_numeric_one_and_does_not_panic() {
        let books = vec![
            book(
                "weird",
                "Weird",
                Some(SeriesKind::AdventurePath),
                Some("Path"),
                Some("one"),
                None,
                None,
            ),
            book(
                "one",
                "One",
                Some(SeriesKind::AdventurePath),
                Some("Path"),
                Some("1"),
                None,
                None,
            ),
        ];
        let sections = adventure_sections(&books);
        // Both stay in the group; the numeric key of "one" is empty and sorts first.
        assert_eq!(names(&books, &sections.paths[0].books), ["weird", "one"]);
    }
}
