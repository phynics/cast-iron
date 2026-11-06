//! Priority tiers for the Books page. One table maps the reviewed publication category
//! (`SourceInfo.category`, from `sources/canonical/publication-metadata.toml`) to a tier.
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

    /// The position of a category inside the table: the order that the Category sort uses
    /// within a tier. Unrecognized values and `None` come last.
    pub fn category_rank(category: Option<&str>) -> usize {
        category
            .and_then(|value| {
                Self::CATEGORIES
                    .iter()
                    .position(|(known, _)| *known == value)
            })
            .unwrap_or(Self::CATEGORIES.len())
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

    /// Every tier but Rulebooks is a disclosure in the search Publication filter. Rulebooks
    /// matter most and lead the list, so they are a plain group that is always listed.
    pub fn is_disclosure(self) -> bool {
        self != Tier::Rulebooks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// The reviewed metadata file is the input. A category value that this table does not
    /// know would silently land in Other, so the test fails and names it instead.
    #[test]
    fn every_reviewed_category_has_a_tier_and_none_falls_to_other_by_accident() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../sources/canonical/publication-metadata.toml"
        );
        let text = std::fs::read_to_string(path).expect("publication-metadata.toml is readable");
        let file: toml::Table = text
            .parse()
            .expect("publication-metadata.toml is valid TOML");
        let publications = file
            .get("publication")
            .and_then(toml::Value::as_table)
            .expect("the file has a [publication] table");
        assert!(!publications.is_empty());
        let mut seen = BTreeSet::new();
        for (id, row) in publications {
            let Some(category) = row.get("category").and_then(toml::Value::as_str) else {
                continue;
            };
            seen.insert(category.to_string());
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
        // The table also keeps its promise for the values the file has today.
        for required in [
            "RPG",
            "Player Companion",
            "Campaign Setting",
            "Adventure Path",
            "Module",
        ] {
            assert!(seen.contains(required), "{required} is no longer reviewed");
        }
    }
}
