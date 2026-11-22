//! The sections of a class page, for its section switcher (#170, part A).
//!
//! A class page is one long document. The switcher is a sticky bar of links that jump to the
//! start of a section; the whole page stays in the document (Find in page, screen readers). The
//! five stops are **Overview · Requirements · Features · Archetypes · Spells**, and a class shows
//! only the stops it has:
//!
//! | Stop | Where it starts |
//! |---|---|
//! | Overview | the top of the page: title, summary card, introduction |
//! | Requirements | the top-level heading `Requirements` (a prestige class; #443) |
//! | Features | the first top-level heading that begins `Class Features` |
//! | Archetypes | the top-level heading `Archetypes` |
//! | Spells | the `Spell list` panel, which exists when the class has a spell list |
//!
//! The mapping reads the headings that the corpus already types, so no entry id, anchor or
//! stored file changes (rule 3). A stop has no entry when its heading is missing.

use cast_iron_schema::{Block, headings::keyed_headings, headings::plain_text};

/// A stop after Overview. Overview is the page itself, so it has no value of its own: the route
/// of the Overview link has no `s=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Requirements,
    Features,
    Archetypes,
    Spells,
}

impl Section {
    /// The stops in the order the switcher shows them, after Overview.
    pub const ALL: [Section; 4] = [
        Section::Requirements,
        Section::Features,
        Section::Archetypes,
        Section::Spells,
    ];

    /// The `s=` value of the address, and the `data-class-section` of the target element.
    pub fn slug(self) -> &'static str {
        match self {
            Section::Requirements => "requirements",
            Section::Features => "features",
            Section::Archetypes => "archetypes",
            Section::Spells => "spells",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Section::Requirements => "Requirements",
            Section::Features => "Features",
            Section::Archetypes => "Archetypes",
            Section::Spells => "Spells",
        }
    }

    /// `overview` and an unknown value read as no section: the page opens at the top.
    pub fn from_slug(slug: &str) -> Option<Section> {
        Section::ALL
            .into_iter()
            .find(|section| section.slug().eq_ignore_ascii_case(slug.trim()))
    }
}

/// The sections that one class page has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sections {
    /// The heading key where Requirements starts.
    requirements: Option<String>,
    /// The heading key (the `h-` id without its prefix) where Features starts.
    features: Option<String>,
    /// The heading key where Archetypes starts.
    archetypes: Option<String>,
    /// Whether the page has a `Spell list` panel to jump to.
    spells: bool,
}

/// How many stops, Overview included, a switcher needs. A class with only Overview and Features
/// has a short page, and two segments would add a jump it hardly needs; it keeps whatever contents
/// disclosure it has. A prestige class has Requirements as well, so it gets the bar: its
/// introduction can run to 2,000 px at 390 px before the Requirements (#443).
const MIN_STOPS: usize = 3;

impl Sections {
    /// Reads the stops from the body that the page renders. `body` must be the slice that the
    /// renderer gets, because the heading keys are numbered across it. `has_spell_list` is true
    /// when the page draws a `Spell list` panel.
    pub fn read(body: &[Block], has_spell_list: bool) -> Self {
        let headings: Vec<(u8, String, String)> = keyed_headings(body)
            .into_iter()
            .filter_map(|(block, key)| match block {
                Block::Heading { level, content, .. } => Some((*level, plain_text(content), key?)),
                _ => None,
            })
            .collect();
        // The stops are headings of the shallowest level: an archetype printed under
        // `Archetypes` has its own `Class Features` heading further down.
        let top = headings.iter().map(|(level, ..)| *level).min();
        let top_level = |wanted: &dyn Fn(&str) -> bool| {
            headings
                .iter()
                .find(|(level, title, _)| Some(*level) == top && wanted(&title.to_lowercase()))
                .map(|(.., key)| key.clone())
        };
        Self {
            requirements: top_level(&|title| title.trim() == "requirements"),
            features: top_level(&|title| title.trim().starts_with("class features")),
            archetypes: top_level(&|title| title.trim() == "archetypes"),
            spells: has_spell_list,
        }
    }

    /// The heading key of the Archetypes section, whether or not the page has a switcher.
    pub fn archetypes_key(&self) -> Option<&str> {
        self.archetypes.as_deref()
    }

    pub fn has(&self, section: Section) -> bool {
        match section {
            Section::Requirements => self.requirements.is_some(),
            Section::Features => self.features.is_some(),
            Section::Archetypes => self.archetypes.is_some(),
            Section::Spells => self.spells,
        }
    }

    /// The stops after Overview that this class has, in switcher order.
    pub fn present(&self) -> Vec<Section> {
        Section::ALL
            .into_iter()
            .filter(|section| self.has(*section))
            .collect()
    }

    /// Whether the page gets a switcher. When it does not, the page keeps its contents disclosure.
    pub fn worth_a_switcher(&self) -> bool {
        self.present().len() + 1 >= MIN_STOPS
    }

    /// `(heading key, section)` of each body heading that a stop starts at, for the renderer.
    /// Spells has no body heading: its target is the panel.
    pub fn body_marks(&self) -> Vec<(String, Section)> {
        [
            (self.requirements.as_ref(), Section::Requirements),
            (self.features.as_ref(), Section::Features),
            (self.archetypes.as_ref(), Section::Archetypes),
        ]
        .into_iter()
        .filter_map(|(key, section)| Some((key?.clone(), section)))
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::Inline;

    fn h(level: u8, title: &str, anchor: Option<&str>) -> Block {
        Block::Heading {
            level,
            anchor: anchor.map(String::from),
            content: vec![Inline::text(title)],
        }
    }

    fn p(text: &str) -> Block {
        Block::Paragraph {
            style: cast_iron_schema::ParagraphStyle::Normal,
            content: vec![Inline::text(text)],
        }
    }

    /// A caster's body in the corpus's shape: features, capstones, archetypes (each with its own
    /// deeper headings), the matrix and the favored class bonuses.
    fn wizard() -> Vec<Block> {
        vec![
            p("Role: ..."),
            h(
                2,
                "Class Features and Spells Per Day",
                Some("class-features"),
            ),
            h(3, "Arcane Bond (Ex or Sp)", Some("arcane-bond")),
            h(2, "Alternate Capstones", None),
            h(2, "Archetypes", None),
            h(3, "Class Features", None),
            h(2, "Archetypes by Feature", None),
            h(2, "Favored Class Bonuses", None),
        ]
    }

    #[test]
    fn a_caster_has_every_stop() {
        let sections = Sections::read(&wizard(), true);
        assert_eq!(
            sections.present(),
            [Section::Features, Section::Archetypes, Section::Spells]
        );
        assert!(sections.worth_a_switcher());
    }

    #[test]
    fn a_non_caster_has_no_spells_stop() {
        let sections = Sections::read(&wizard(), false);
        assert_eq!(sections.present(), [Section::Features, Section::Archetypes]);
        assert!(!sections.has(Section::Spells));
        assert!(sections.worth_a_switcher());
    }

    #[test]
    fn the_stops_start_at_the_top_level_headings_not_an_archetypes_own() {
        // The deeper `Class Features` under `Archetypes` is not the Features stop, and
        // `Archetypes by Feature` is not the Archetypes stop.
        let marks = Sections::read(&wizard(), true).body_marks();
        assert_eq!(
            marks,
            [
                ("class-features".to_string(), Section::Features),
                ("sec-archetypes".to_string(), Section::Archetypes),
            ]
        );
    }

    #[test]
    fn the_first_class_features_heading_wins() {
        let body = vec![
            h(2, "Class Features", Some("a")),
            h(2, "Class Features and Spells Per Day", Some("b")),
        ];
        assert_eq!(
            Sections::read(&body, false).body_marks(),
            [("a".to_string(), Section::Features)]
        );
    }

    #[test]
    fn a_prestige_class_has_requirements_and_features() {
        // Requirements, Class Skills, Class Features: Overview, Requirements and Features are
        // three stops, so the page gets a switcher (#443).
        let prestige = vec![
            p("A long introduction."),
            h(3, "Requirements", Some("requirements")),
            h(3, "Class Skills", None),
            h(3, "Class Features", Some("class-features")),
        ];
        let sections = Sections::read(&prestige, false);
        assert_eq!(
            sections.present(),
            [Section::Requirements, Section::Features]
        );
        assert!(sections.worth_a_switcher());
        assert_eq!(
            sections.body_marks(),
            [
                ("requirements".to_string(), Section::Requirements),
                ("class-features".to_string(), Section::Features),
            ]
        );
    }

    #[test]
    fn requirements_is_a_stop_only_at_the_top_level_and_with_its_exact_title() {
        // A deeper `Requirements` (an archetype's, under a top-level heading) and a heading that
        // only mentions the word are not stops.
        let body = vec![
            h(2, "Class Features", None),
            h(3, "Requirements", None),
            h(2, "Requirements for Entry", None),
        ];
        let sections = Sections::read(&body, false);
        assert!(!sections.has(Section::Requirements));
        assert_eq!(sections.present(), [Section::Features]);
    }

    #[test]
    fn a_stop_without_its_heading_is_left_out() {
        // A prestige class without a Requirements heading has Overview and Features: too few for
        // a switcher.
        let prestige = vec![h(2, "Class Skills", None), h(2, "Class Features", None)];
        let sections = Sections::read(&prestige, false);
        assert_eq!(sections.present(), [Section::Features]);
        assert!(!sections.worth_a_switcher());
        // A body with no headings has no stops.
        let none = Sections::read(&[p("Only text")], false);
        assert!(none.present().is_empty());
        assert!(!none.worth_a_switcher());
        // A spell list without a Features heading is Overview and Spells: too few again.
        let list_only = Sections::read(&[p("text")], true);
        assert_eq!(list_only.present(), [Section::Spells]);
        assert!(!list_only.worth_a_switcher());
    }

    #[test]
    fn a_heading_that_only_mentions_the_words_is_not_a_stop() {
        let body = vec![
            h(2, "Favored Class Bonuses", None),
            h(2, "Archetypes by Feature", None),
            h(2, "Using Class Features", None),
        ];
        assert!(Sections::read(&body, false).present().is_empty());
    }

    #[test]
    fn a_duplicate_heading_uses_its_own_key() {
        // The keys come from the shared numbering, so a repeated `Archetypes` stays distinct.
        let body = vec![
            h(2, "Archetypes", Some("archetypes")),
            h(2, "Archetypes", Some("archetypes")),
        ];
        assert_eq!(
            Sections::read(&body, false).body_marks(),
            [("archetypes".to_string(), Section::Archetypes)]
        );
    }

    #[test]
    fn slugs_round_trip_and_overview_is_no_section() {
        for section in Section::ALL {
            assert_eq!(Section::from_slug(section.slug()), Some(section));
        }
        assert_eq!(Section::from_slug("Archetypes"), Some(Section::Archetypes));
        assert_eq!(Section::from_slug("overview"), None);
        assert_eq!(Section::from_slug(""), None);
        assert_eq!(Section::from_slug("nope"), None);
    }
}
