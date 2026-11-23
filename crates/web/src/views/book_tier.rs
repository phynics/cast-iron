//! Priority tiers for the Books page. One table maps the reviewed publication category
//! (`SourceInfo.category`, from the corpus `publications.toml`) to a tier.
//! Core rulebooks matter most; adventures matter least. The mapping is pure so native
//! tests cover it, including every category value that the reviewed file contains.

use cast_iron_schema::{EntryStub, SeriesKind, SourceInfo};
use cast_iron_search::NEUTRAL_SOURCE_RANK;

/// The search tie-break rank of every source in a catalog, from its reviewed category. Hand the
/// result to `SearchIndex::set_source_ranks`.
pub fn source_ranks(sources: &[SourceInfo]) -> Vec<(&str, u8)> {
    sources
        .iter()
        .map(|source| (source.id.as_str(), Tier::of(source).search_rank()))
        .collect()
}

/// Where a printing sits among the printings of one rule: the product line of its primary book,
/// then how new its books are. Search tie-breaks and list folds both sort by this one value, so
/// the two cannot drift. `Ord` puts the printing to read first lowest: a lower rank first, then
/// a newer printing, so an undated entry (`newest` 0) sorts after every dated one of its rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrintingOrder {
    /// [`Tier::search_rank`] of the entry's primary book, as search ranks the document.
    pub rank: u8,
    /// The dense rank of the newest-dated member book. A larger value is a newer printing; 0
    /// means that no member book has a date.
    pub newest: u32,
}

impl PrintingOrder {
    /// The order of an entry that no book of the catalog describes: neutral rank, undated.
    pub const UNKNOWN: PrintingOrder = PrintingOrder {
        rank: NEUTRAL_SOURCE_RANK,
        newest: 0,
    };
}

impl PartialOrd for PrintingOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrintingOrder {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank
            .cmp(&other.rank)
            .then_with(|| other.newest.cmp(&self.newest))
    }
}

/// The per-book facts behind [`PrintingOrder`]: each book's search rank and the dense rank of
/// its publication date. Build it once per catalog; asking for an entry's order then reads two
/// small maps.
///
/// The values are dense ranks of the distinct dates (the date strings are ISO 8601, which sort
/// as text). A book with no date, or an empty one, has no date rank.
#[derive(Debug, Clone, Default)]
pub struct PrintingOrders {
    book_rank: std::collections::HashMap<String, u8>,
    book_date: std::collections::HashMap<String, u32>,
}

impl PrintingOrders {
    pub fn new(sources: &[SourceInfo]) -> Self {
        let dates: std::collections::BTreeSet<&str> = sources
            .iter()
            .filter_map(|source| source.publication_date.as_deref())
            .filter(|date| !date.is_empty())
            .collect();
        let rank_of_date: std::collections::BTreeMap<&str, u32> =
            dates.into_iter().zip(1u32..).collect();
        Self {
            book_rank: source_ranks(sources)
                .into_iter()
                .map(|(id, rank)| (id.to_owned(), rank))
                .collect(),
            book_date: sources
                .iter()
                .filter_map(|source| {
                    let date = source.publication_date.as_deref()?;
                    Some((source.id.clone(), *rank_of_date.get(date)?))
                })
                .collect(),
        }
    }

    /// The order of an entry. Its rank is that of its primary book (`stub.source`), as search
    /// ranks the document. Its recency is that of the newest-dated book among its member books,
    /// so a rule kept in several books ranks by its latest printing. A book that revises the
    /// rule is a different entry and its date does not count here.
    pub fn of(&self, stub: &EntryStub) -> PrintingOrder {
        let rank = self
            .book_rank
            .get(stub.source.as_str())
            .copied()
            .unwrap_or(NEUTRAL_SOURCE_RANK);
        let newest = stub
            .member_books()
            .iter()
            .filter_map(|book| self.book_date.get(book.as_str()))
            .copied()
            .max()
            .unwrap_or(0);
        PrintingOrder { rank, newest }
    }
}

/// The printing recency of every entry, from the catalog `publication_date` of its books. Hand
/// the result to `SearchIndex::set_printing_recency`. It is [`PrintingOrders::of`] read for its
/// `newest` field, so search and list folds share one definition. An undated entry is left out
/// of the result.
pub fn printing_recency<'a>(
    orders: &PrintingOrders,
    entries: &'a [EntryStub],
) -> Vec<(&'a str, u32)> {
    entries
        .iter()
        .filter_map(|stub| {
            let newest = orders.of(stub).newest;
            (newest > 0).then_some((stub.id.as_str(), newest))
        })
        .collect()
}

/// A group of publications on the Books page, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Rulebooks,
    PlayerCompanions,
    CampaignSetting,
    Adventures,
    /// `Miscellaneous`, any category this table does not know, and a publication with no
    /// reviewed category.
    Other,
    /// A publication the corpus marks `third_party = true` (#189), whatever its category.
    ThirdParty,
}

impl Tier {
    pub const ALL: [Tier; 6] = [
        Tier::Rulebooks,
        Tier::PlayerCompanions,
        Tier::CampaignSetting,
        Tier::Adventures,
        Tier::Other,
        Tier::ThirdParty,
    ];

    /// Every category value the table knows, with the tier that it belongs to. A value missing
    /// here falls to [`Tier::Other`]. `Miscellaneous` is listed on purpose: it is a reviewed
    /// value that belongs in Other, not an unrecognized one.
    pub const CATEGORIES: [(&'static str, Tier); 8] = [
        ("RPG", Tier::Rulebooks),
        ("Player Companion", Tier::PlayerCompanions),
        ("Campaign Setting", Tier::CampaignSetting),
        ("Adventure Path", Tier::Adventures),
        ("Module", Tier::Adventures),
        ("Society", Tier::Adventures),
        ("Miscellaneous", Tier::Other),
        ("Web supplement", Tier::Other),
    ];

    /// The tier for a publication: the third-party marker first, else its category. A
    /// recognized series kind with no reviewed category still belongs to Adventures: the
    /// pinned Society scenario carries `series_kind = "society"` and no category.
    pub fn of(source: &SourceInfo) -> Tier {
        Self::of_parts(
            source.third_party,
            source.category.as_deref(),
            source.series_kind,
        )
    }

    /// [`Tier::of`] over the fields a list row keeps, so the Books and Manage books rows group
    /// a marked book without holding a whole [`SourceInfo`].
    pub fn of_parts(
        third_party: bool,
        category: Option<&str>,
        series_kind: Option<SeriesKind>,
    ) -> Tier {
        if third_party {
            Tier::ThirdParty
        } else {
            match Self::from_category(category) {
                Tier::Other if series_kind.is_some() => Tier::Adventures,
                tier => tier,
            }
        }
    }

    /// The tier for a publication's category. `None` (no reviewed category) and any value that
    /// is not in [`Tier::CATEGORIES`] are Other. This ignores the third-party marker; use
    /// [`Tier::of`] for a whole source.
    pub fn from_category(category: Option<&str>) -> Tier {
        match category {
            Some(value) => Self::CATEGORIES
                .iter()
                .find(|(known, _)| *known == value)
                .map_or(Tier::Other, |(_, tier)| *tier),
            None => Tier::Other,
        }
    }

    /// The tie-break rank that search uses: lower sorts first on equal relevance. Rulebooks are
    /// 0, and third-party books follow every Paizo tier. Other takes the search crate's neutral
    /// rank, so a source with no reviewed category sorts like one that search has no rank for.
    /// Search receives this number as plain data and never sees `Tier`.
    pub fn search_rank(self) -> u8 {
        match self {
            Tier::Rulebooks => 0,
            Tier::PlayerCompanions => 1,
            Tier::CampaignSetting => 2,
            Tier::Adventures => 3,
            Tier::ThirdParty => 4,
            Tier::Other => NEUTRAL_SOURCE_RANK,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tier::Rulebooks => "Rulebooks",
            Tier::PlayerCompanions => "Player Companions",
            Tier::CampaignSetting => "Campaign Setting",
            Tier::Adventures => "Adventures",
            Tier::Other => "Other",
            Tier::ThirdParty => "Third party",
        }
    }

    /// Adventures and third-party books start closed in the search Publication filter: there
    /// are many adventures and they matter least, and third-party books are opt-in (#189).
    pub fn collapsed_by_default(self) -> bool {
        matches!(self, Tier::Adventures | Tier::ThirdParty)
    }

    /// On Books and Manage books only Rulebooks start open: the others are one tap away, and
    /// the first screen shows the books that matter most. A reader's choice replaces this for
    /// the rest of the session ([`OpenGroups`]).
    pub fn open_by_default(self) -> bool {
        self == Tier::Rulebooks
    }

    /// Every tier but Rulebooks is a disclosure in the search Publication filter. Rulebooks
    /// matter most and lead the list, so they are a plain group that is always listed.
    pub fn is_disclosure(self) -> bool {
        self != Tier::Rulebooks
    }
}

/// Which groups on Books and Manage books the reader has opened or closed. It is kept in memory
/// for the session, so Back to Books finds the same groups open before the page restores the
/// scroll position: a position inside an opened group would otherwise land on closed rows.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OpenGroups {
    chosen: std::collections::BTreeMap<Tier, bool>,
}

impl OpenGroups {
    pub fn is_open(&self, tier: Tier) -> bool {
        self.chosen
            .get(&tier)
            .copied()
            .unwrap_or_else(|| tier.open_by_default())
    }

    pub fn set(&mut self, tier: Tier, open: bool) {
        self.chosen.insert(tier, open);
    }
}

thread_local! {
    static OPEN_GROUPS: std::cell::RefCell<OpenGroups> = std::cell::RefCell::new(OpenGroups::default());
}

/// Whether a group starts open on this visit: the reader's last choice, else the default.
pub fn group_is_open(tier: Tier) -> bool {
    OPEN_GROUPS.with(|groups| groups.borrow().is_open(tier))
}

/// Records that the reader opened or closed a group.
pub fn remember_group(tier: Tier, open: bool) {
    OPEN_GROUPS.with(|groups| groups.borrow_mut().set(tier, open));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_rulebooks_start_open_and_a_choice_replaces_that() {
        let mut groups = OpenGroups::default();
        for tier in Tier::ALL {
            assert_eq!(groups.is_open(tier), tier == Tier::Rulebooks, "{tier:?}");
        }
        groups.set(Tier::Adventures, true);
        groups.set(Tier::Rulebooks, false);
        assert!(groups.is_open(Tier::Adventures));
        assert!(!groups.is_open(Tier::Rulebooks));
        // The other groups keep their default.
        assert!(!groups.is_open(Tier::Other));
        groups.set(Tier::Adventures, false);
        assert!(!groups.is_open(Tier::Adventures));
    }

    #[test]
    fn the_session_remembers_each_group_it_was_told_about() {
        remember_group(Tier::CampaignSetting, true);
        assert!(group_is_open(Tier::CampaignSetting));
        remember_group(Tier::CampaignSetting, false);
        assert!(!group_is_open(Tier::CampaignSetting));
    }
    use std::collections::BTreeSet;

    #[test]
    fn categories_map_to_the_priority_order() {
        let map = |value: &str| Tier::from_category(Some(value));
        assert_eq!(map("RPG"), Tier::Rulebooks);
        assert_eq!(map("Player Companion"), Tier::PlayerCompanions);
        assert_eq!(map("Campaign Setting"), Tier::CampaignSetting);
        for adventure in ["Adventure Path", "Module", "Society"] {
            assert_eq!(map(adventure), Tier::Adventures, "{adventure}");
        }
        assert_eq!(map("Miscellaneous"), Tier::Other);
        assert_eq!(map("Web supplement"), Tier::Other);
        assert_eq!(map("Something new"), Tier::Other);
        assert_eq!(Tier::from_category(None), Tier::Other);
    }

    fn source(id: &str, category: Option<&str>) -> SourceInfo {
        SourceInfo {
            share_slot: None,
            id: id.into(),
            title: id.into(),
            aon: None,
            entry_count: 1,
            copyright: None,
            category: category.map(String::from),
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
        }
    }

    /// Search ranks follow the tier order, with Other neutral and third-party last of the
    /// ranked tiers. This ties the search tie-break to the Books grouping: one table,
    /// `Tier::CATEGORIES`, feeds both.
    #[test]
    fn search_ranks_follow_the_tier_order_and_other_is_neutral() {
        assert_eq!(
            [
                Tier::Rulebooks.search_rank(),
                Tier::PlayerCompanions.search_rank(),
                Tier::CampaignSetting.search_rank(),
                Tier::Adventures.search_rank(),
                Tier::ThirdParty.search_rank(),
            ],
            [0, 1, 2, 3, 4]
        );
        assert_eq!(Tier::Other.search_rank(), NEUTRAL_SOURCE_RANK);
        assert_eq!(
            Tier::ALL,
            [
                Tier::Rulebooks,
                Tier::PlayerCompanions,
                Tier::CampaignSetting,
                Tier::Adventures,
                Tier::Other,
                Tier::ThirdParty
            ]
        );
        // Every category value in the table agrees with its tier.
        for (category, tier) in Tier::CATEGORIES {
            let sources = [source("x", Some(category))];
            assert_eq!(source_ranks(&sources), [("x", tier.search_rank())]);
        }
        // The third-party marker wins over the category, and it is its own group.
        let mut marked = source("indie", Some("RPG"));
        marked.third_party = true;
        assert_eq!(Tier::of(&marked), Tier::ThirdParty);
        assert_eq!(source_ranks(&[marked]), [("indie", 4)]);
        assert_eq!(Tier::ThirdParty.label(), "Third party");
        assert!(Tier::ThirdParty.collapsed_by_default());
        assert!(Tier::ThirdParty.is_disclosure());
    }

    #[test]
    fn a_series_kind_without_a_category_belongs_to_adventures() {
        let mut society = source("society", None);
        society.series_kind = Some(SeriesKind::Society);
        assert_eq!(Tier::of(&society), Tier::Adventures);
        let mut module = source("module", None);
        module.series_kind = Some(SeriesKind::Module);
        assert_eq!(Tier::of(&module), Tier::Adventures);
        // The reviewed category still decides a book with a recognized series kind.
        let mut companion = source("companion", Some("Player Companion"));
        companion.series_kind = Some(SeriesKind::Module);
        assert_eq!(Tier::of(&companion), Tier::PlayerCompanions);
        // The third-party marker still outweighs both.
        let mut indie = source("indie", None);
        indie.series_kind = Some(SeriesKind::Society);
        indie.third_party = true;
        assert_eq!(Tier::of(&indie), Tier::ThirdParty);
        assert_eq!(Tier::from_category(None), Tier::Other);
    }

    #[test]
    fn source_ranks_treat_a_missing_or_unknown_category_as_other() {
        let sources = [
            source("core", Some("RPG")),
            source("pc", Some("Player Companion")),
            source("ap", Some("Adventure Path")),
            source("none", None),
            source("new", Some("Something new")),
        ];
        assert_eq!(
            source_ranks(&sources),
            [
                ("core", 0),
                ("pc", 1),
                ("ap", 3),
                ("none", NEUTRAL_SOURCE_RANK),
                ("new", NEUTRAL_SOURCE_RANK)
            ]
        );
    }

    fn dated(id: &str, date: Option<&str>) -> SourceInfo {
        let mut info = source(id, Some("RPG"));
        info.publication_date = date.map(String::from);
        info
    }

    fn stub(id: &str, books: &[&str]) -> EntryStub {
        EntryStub {
            id: cast_iron_schema::EntryId::parse(id).unwrap(),
            kind: cast_iron_schema::EntryKind::Feat,
            title: id.to_string(),
            source: books[0].to_string(),
            books: EntryStub::encode_books(
                &books
                    .iter()
                    .map(|book| book.to_string())
                    .collect::<Vec<_>>(),
            ),
            parent: None,
            shard: "entries/feat-000.json".into(),
        }
    }

    #[test]
    fn printing_recency_is_the_newest_dated_member_book() {
        let sources = [
            dated("old", Some("2009-08-01")),
            dated("mid", Some("2011-03-01")),
            dated("new", Some("2017-04-26")),
            dated("undated", None),
        ];
        let entries = [
            stub("ref/feat/lone-old", &["old"]),
            stub("ref/feat/lone-new", &["new"]),
            // The primary book is the oldest, but the entry is also printed in `new`.
            stub("ref/feat/reprinted", &["old", "new", "mid"]),
            stub("ref/feat/undated-first", &["undated", "mid"]),
            stub("ref/feat/undated", &["undated"]),
            stub("ref/feat/unknown-book", &["not-in-catalog"]),
        ];
        assert_eq!(
            printing_recency(&PrintingOrders::new(&sources), &entries),
            [
                ("ref/feat/lone-old", 1),
                ("ref/feat/lone-new", 3),
                ("ref/feat/reprinted", 3),
                ("ref/feat/undated-first", 2),
            ]
        );
    }

    #[test]
    fn printing_recency_ranks_equal_dates_alike_and_ignores_empty_dates() {
        let sources = [
            dated("a", Some("2012-01-01")),
            dated("b", Some("2012-01-01")),
            dated("c", Some("")),
            dated("d", Some("2013-02-01")),
        ];
        let entries = [
            stub("ref/feat/a", &["a"]),
            stub("ref/feat/b", &["b"]),
            stub("ref/feat/c", &["c"]),
            stub("ref/feat/d", &["d"]),
        ];
        assert_eq!(
            printing_recency(&PrintingOrders::new(&sources), &entries),
            [("ref/feat/a", 1), ("ref/feat/b", 1), ("ref/feat/d", 2)]
        );
        assert!(printing_recency(&PrintingOrders::new(&[]), &[]).is_empty());
    }

    fn book(id: &str, category: &str, date: Option<&str>) -> SourceInfo {
        let mut info = source(id, Some(category));
        info.publication_date = date.map(String::from);
        info
    }

    /// The Tent case of #637: one item in a rulebook, a companion-line reprint and a later
    /// equipment book. The newest printing leads within one product line.
    #[test]
    fn the_newest_printing_of_one_product_line_comes_first() {
        let sources = [
            book("core", "RPG", Some("2009-08-01")),
            book("apg", "RPG", Some("2010-08-01")),
            book("ue", "RPG", Some("2012-08-01")),
        ];
        let orders = PrintingOrders::new(&sources);
        let mut stubs = [
            stub("ref/item/tent-apg", &["apg"]),
            stub("ref/item/tent-core", &["core"]),
            stub("ref/item/tent-ue", &["ue"]),
        ];
        stubs.sort_by_key(|s| orders.of(s));
        let ids: Vec<&str> = stubs.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "ref/item/tent-ue",
                "ref/item/tent-apg",
                "ref/item/tent-core"
            ]
        );
    }

    #[test]
    fn an_undated_printing_sorts_after_every_dated_one_of_its_rank() {
        let sources = [
            book("old", "RPG", Some("2009-08-01")),
            book("none", "RPG", None),
        ];
        let orders = PrintingOrders::new(&sources);
        let dated = orders.of(&stub("ref/feat/a", &["old"]));
        let undated = orders.of(&stub("ref/feat/b", &["none"]));
        assert_eq!(undated.newest, 0);
        assert!(dated < undated);
        assert_eq!(
            orders.of(&stub("ref/feat/c", &["missing"])),
            PrintingOrder::UNKNOWN
        );
    }

    #[test]
    fn the_product_line_rank_wins_over_recency() {
        let sources = [
            book("core", "RPG", Some("2009-08-01")),
            book("pc", "Player Companion", Some("2015-01-01")),
            book("ap", "Adventure Path", Some("2016-01-01")),
        ];
        let orders = PrintingOrders::new(&sources);
        let mut stubs = [
            stub("ref/feat/ap", &["ap"]),
            stub("ref/feat/pc", &["pc"]),
            stub("ref/feat/core", &["core"]),
        ];
        stubs.sort_by_key(|s| orders.of(s));
        let ids: Vec<&str> = stubs.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["ref/feat/core", "ref/feat/pc", "ref/feat/ap"]);
    }

    /// The rank is the primary book's; the recency is the newest member book's.
    #[test]
    fn rank_comes_from_the_primary_book_and_recency_from_every_member_book() {
        let sources = [
            book("core", "RPG", Some("2009-08-01")),
            book("ap", "Adventure Path", Some("2016-01-01")),
        ];
        let orders = PrintingOrders::new(&sources);
        let reprint = orders.of(&stub("ref/feat/x", &["core", "ap"]));
        assert_eq!(reprint, PrintingOrder { rank: 0, newest: 2 });
    }

    /// Search and a list fold take their order from the same value: the keys of
    /// [`PrintingOrders::of`], then title and id, sorted the way search sorts equal scores.
    #[test]
    fn list_keys_and_search_tie_breaks_agree() {
        let sources = [
            book("core", "RPG", Some("2009-08-01")),
            book("ue", "RPG", Some("2012-08-01")),
            book("pc", "Player Companion", Some("2013-01-01")),
            book("none", "RPG", None),
        ];
        let orders = PrintingOrders::new(&sources);
        let stubs = [
            stub("ref/item/tent-core", &["core"]),
            stub("ref/item/tent-ue", &["ue"]),
            stub("ref/item/tent-pc", &["pc"]),
            stub("ref/item/tent-none", &["none"]),
            stub("ref/item/tent-pc2", &["pc"]),
        ];
        let mut by_list: Vec<&EntryStub> = stubs.iter().collect();
        by_list.sort_by(|a, b| {
            orders
                .of(a)
                .cmp(&orders.of(b))
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.id.cmp(&b.id))
        });
        let recency: std::collections::HashMap<&str, u32> =
            printing_recency(&orders, &stubs).into_iter().collect();
        let ranks: std::collections::HashMap<&str, u8> =
            source_ranks(&sources).into_iter().collect();
        // Search's keys: source rank ascending, recency descending, title, id.
        let mut by_search: Vec<&EntryStub> = stubs.iter().collect();
        by_search.sort_by(|a, b| {
            let key = |s: &EntryStub| {
                (
                    ranks[s.source.as_str()],
                    std::cmp::Reverse(recency.get(s.id.as_str()).copied().unwrap_or(0)),
                )
            };
            key(a)
                .cmp(&key(b))
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.id.cmp(&b.id))
        });
        let list_ids: Vec<&str> = by_list.iter().map(|s| s.id.as_str()).collect();
        let search_ids: Vec<&str> = by_search.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(list_ids, search_ids);
        assert_eq!(
            list_ids,
            [
                "ref/item/tent-ue",
                "ref/item/tent-core",
                "ref/item/tent-none",
                "ref/item/tent-pc",
                "ref/item/tent-pc2",
            ]
        );
    }

    #[test]
    fn tiers_are_declared_in_priority_order() {
        let mut sorted = Tier::ALL;
        sorted.sort();
        assert_eq!(sorted, Tier::ALL);
        let labels: Vec<_> = Tier::ALL.iter().map(|tier| tier.label()).collect();
        assert_eq!(
            labels,
            [
                "Rulebooks",
                "Player Companions",
                "Campaign Setting",
                "Adventures",
                "Other",
                "Third party"
            ]
        );
        assert!(Tier::Adventures.collapsed_by_default());
        assert!(Tier::ThirdParty.collapsed_by_default());
        assert_eq!(
            Tier::ALL
                .iter()
                .filter(|tier| tier.collapsed_by_default())
                .count(),
            2
        );
    }

    #[test]
    fn every_category_in_the_table_belongs_to_exactly_one_tier() {
        let names: BTreeSet<_> = Tier::CATEGORIES.iter().map(|(name, _)| *name).collect();
        assert_eq!(names.len(), Tier::CATEGORIES.len(), "a duplicate category");
    }

    /// The categories that the pinned corpus (`publications.toml`) uses. A category value that
    /// this table does not know would silently land in Other, so the test fails and names it
    /// instead. When a corpus bump adds a category, add it here and to `Tier::CATEGORIES`.
    const REVIEWED_CATEGORIES: [&str; 6] = [
        "Adventure Path",
        "Campaign Setting",
        "Module",
        "Player Companion",
        "RPG",
        "Web supplement",
    ];

    fn assert_category_has_a_tier(id: &str, category: &str) {
        let known = Tier::CATEGORIES.iter().find(|(name, _)| *name == category);
        let (_, tier) = known.unwrap_or_else(|| {
            panic!(
                "publication {id} has category {category:?}, which Tier::CATEGORIES does not \
                 list. Add it with its tier; do not let it fall to Other."
            )
        });
        assert_eq!(Tier::from_category(Some(category)), *tier);
        if *tier == Tier::Other {
            assert!(
                ["Miscellaneous", "Web supplement"].contains(&category),
                "publication {id} falls to Other unexpectedly"
            );
        }
    }

    #[test]
    fn every_reviewed_category_has_a_tier_and_none_falls_to_other_by_accident() {
        for category in REVIEWED_CATEGORIES {
            assert_category_has_a_tier("(reviewed list)", category);
        }
    }

    /// Every category the pinned corpus uses must be in the reviewed list above. The corpus is
    /// read from the cache that `cargo xtask fetch-sources` fills. CI (the `CI` variable is set)
    /// fails when the cache is missing; a local run without it prints a skip message.
    #[test]
    fn every_category_in_the_cached_corpus_is_reviewed() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../.cache/sources/cast-iron-data/publications.toml"
        );
        let Ok(text) = std::fs::read_to_string(path) else {
            assert!(
                std::env::var_os("CI").is_none(),
                "the pinned corpus is not in .cache/sources/cast-iron-data; the CI job must run \
                 `cargo xtask fetch-sources` first"
            );
            eprintln!("skipped: run `cargo xtask fetch-sources` to check the cached corpus");
            return;
        };
        let file: toml::Table = text.parse().expect("publications.toml is valid TOML");
        let rows = file
            .get("publication")
            .and_then(toml::Value::as_array)
            .expect("the file has [[publication]] rows");
        assert!(!rows.is_empty());
        for row in rows {
            let id = row.get("id").and_then(toml::Value::as_str).unwrap_or("?");
            let Some(category) = row.get("category").and_then(toml::Value::as_str) else {
                continue;
            };
            assert!(
                REVIEWED_CATEGORIES.contains(&category),
                "publication {id} has category {category:?}; add it to REVIEWED_CATEGORIES"
            );
            assert_category_has_a_tier(id, category);
        }
    }
}
