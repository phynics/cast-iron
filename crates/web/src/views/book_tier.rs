//! Priority tiers for the Books page. One table maps the reviewed publication category
//! (`SourceInfo.category`, from the corpus `publications.toml`) to a tier.
//! Core rulebooks matter most; adventures matter least. The mapping is pure so native
//! tests cover it, including every category value that the reviewed file contains.

use cast_iron_schema::SourceInfo;
use cast_iron_search::NEUTRAL_SOURCE_RANK;

/// The search tie-break rank of every source in a catalog, from its reviewed category. Hand the
/// result to `SearchIndex::set_source_ranks`.
pub fn source_ranks(sources: &[SourceInfo]) -> Vec<(&str, u8)> {
    sources
        .iter()
        .map(|source| {
            (
                source.id.as_str(),
                Tier::from_category(source.category.as_deref()).search_rank(),
            )
        })
        .collect()
}

/// The source a reader should see first. An entry's first source wins while it is active, so the
/// label is unchanged when every book is active. When the first source is inactive, choose an
/// active source in the highest-priority tier ([`Tier::ALL`] order), keeping the caller's order
/// inside a tier. With every source inactive, keep the first so the label still names one.
pub fn preferred_source<'a>(
    first: &'a str,
    alternatives: impl IntoIterator<Item = &'a str>,
    is_active: impl Fn(&str) -> bool,
    tier_of: impl Fn(&str) -> Tier,
) -> &'a str {
    if is_active(first) {
        return first;
    }
    alternatives
        .into_iter()
        .filter(|id| *id != first && is_active(id))
        .min_by_key(|id| tier_of(id))
        .unwrap_or(first)
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
}

impl Tier {
    pub const ALL: [Tier; 5] = [
        Tier::Rulebooks,
        Tier::PlayerCompanions,
        Tier::CampaignSetting,
        Tier::Adventures,
        Tier::Other,
    ];

    /// Every category value the table knows, with the tier that it belongs to. A value missing
    /// here falls to [`Tier::Other`]. `Miscellaneous` is listed on purpose: it is a reviewed
    /// value that belongs in Other, not an unrecognized one.
    pub const CATEGORIES: [(&'static str, Tier); 7] = [
        ("RPG", Tier::Rulebooks),
        ("Player Companion", Tier::PlayerCompanions),
        ("Campaign Setting", Tier::CampaignSetting),
        ("Adventure Path", Tier::Adventures),
        ("Module", Tier::Adventures),
        ("Society", Tier::Adventures),
        ("Miscellaneous", Tier::Other),
    ];

    /// The tier for a publication's category. `None` (no reviewed category) and any value that
    /// is not in [`Tier::CATEGORIES`] are Other.
    pub fn from_category(category: Option<&str>) -> Tier {
        match category {
            Some(value) => Self::CATEGORIES
                .iter()
                .find(|(known, _)| *known == value)
                .map_or(Tier::Other, |(_, tier)| *tier),
            None => Tier::Other,
        }
    }

    /// The tie-break rank that search uses: lower sorts first on equal relevance. It is the
    /// tier's position in [`Tier::ALL`], so Rulebooks are 0. Other takes the search crate's
    /// neutral rank, so a source with no reviewed category sorts like one that search has no
    /// rank for. Search receives this number as plain data and never sees `Tier`.
    pub fn search_rank(self) -> u8 {
        match self {
            Tier::Other => NEUTRAL_SOURCE_RANK,
            tier => Self::ALL
                .iter()
                .position(|known| *known == tier)
                .map_or(NEUTRAL_SOURCE_RANK, |position| position as u8),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tier::Rulebooks => "Rulebooks",
            Tier::PlayerCompanions => "Player Companions",
            Tier::CampaignSetting => "Campaign Setting",
            Tier::Adventures => "Adventures",
            Tier::Other => "Other",
        }
    }

    /// Adventures start closed: there are many of them and they matter least. Every book in
    /// every tier still starts active.
    pub fn collapsed_by_default(self) -> bool {
        self == Tier::Adventures
    }

    /// On Books and Active books only Rulebooks start open: the others are one tap away, and
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

/// Which groups on Books and Active books the reader has opened or closed. It is kept in memory
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
        assert_eq!(map("Something new"), Tier::Other);
        assert_eq!(Tier::from_category(None), Tier::Other);
    }

    fn source(id: &str, category: Option<&str>) -> SourceInfo {
        SourceInfo {
            id: id.into(),
            title: id.into(),
            entry_count: 1,
            copyright: None,
            category: category.map(String::from),
            publication_date: None,
        }
    }

    /// Search ranks follow the tier order, with Other neutral. This ties the search tie-break
    /// to the Books grouping: one table, `Tier::CATEGORIES`, feeds both.
    #[test]
    fn search_ranks_follow_the_tier_order_and_other_is_neutral() {
        let ranks: Vec<u8> = Tier::ALL.iter().map(|tier| tier.search_rank()).collect();
        assert_eq!(ranks, [0, 1, 2, 3, NEUTRAL_SOURCE_RANK]);
        assert!(ranks.windows(2).all(|pair| pair[0] < pair[1]));
        // Every category value in the table agrees with its tier.
        for (category, tier) in Tier::CATEGORIES {
            let sources = [source("x", Some(category))];
            assert_eq!(source_ranks(&sources), [("x", tier.search_rank())]);
        }
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

    #[test]
    fn preferred_source_keeps_an_active_first_source() {
        let tier_of = |id: &str| Tier::from_category(category_of(id));
        assert_eq!(
            preferred_source("pc", ["pc", "core"], |_| true, tier_of),
            "pc"
        );
    }

    #[test]
    fn preferred_source_picks_the_highest_priority_active_source() {
        let tier_of = |id: &str| Tier::from_category(category_of(id));
        let active = |id: &str| matches!(id, "pc" | "core" | "ap");
        // The first source is inactive; Core Rulebook outranks the Companion and the Adventure.
        assert_eq!(
            preferred_source(
                "settings",
                ["settings", "pc", "core", "ap"],
                active,
                tier_of
            ),
            "core"
        );
        // Inside one tier the caller's order decides.
        assert_eq!(
            preferred_source("settings", ["settings", "pc", "pc2"], active, tier_of),
            "pc"
        );
        // With every source inactive, the first still names the entry.
        assert_eq!(
            preferred_source("settings", ["settings", "core"], |_| false, tier_of),
            "settings"
        );
    }

    fn category_of(id: &str) -> Option<&'static str> {
        match id {
            "core" => Some("RPG"),
            "pc" | "pc2" => Some("Player Companion"),
            "settings" => Some("Campaign Setting"),
            "ap" => Some("Adventure Path"),
            _ => None,
        }
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
                "Other"
            ]
        );
        assert!(Tier::Adventures.collapsed_by_default());
        assert!(
            Tier::ALL
                .iter()
                .filter(|tier| tier.collapsed_by_default())
                .count()
                == 1
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
    const REVIEWED_CATEGORIES: [&str; 5] = [
        "Adventure Path",
        "Campaign Setting",
        "Module",
        "Player Companion",
        "RPG",
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
            assert_eq!(
                category, "Miscellaneous",
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
