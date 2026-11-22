//! Spell-row density: Compact or Wide. The choice belongs to the device, sits in IndexedDB
//! with the other reader preferences (key `spell_density`), and survives a reload and offline
//! use. The list container carries it as `data-density`, so CSS selects the layout and the
//! rendering logic does not fork.

use crate::store;

/// The radio group under Settings > Appearance. A spell list has no radio group: it switches with
/// one text button.
pub const SETTINGS_RADIO_GROUP: &str = "spell-density-settings";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Density {
    /// Two lines at every width: name and school, then one fact line that ellipsises.
    Compact,
    /// The #179 default: three lines on a phone, two from 40 rem.
    #[default]
    Wide,
}

impl Density {
    pub const ALL: [Density; 2] = [Density::Compact, Density::Wide];

    pub fn slug(self) -> &'static str {
        match self {
            Density::Compact => "compact",
            Density::Wide => "wide",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|density| density.slug() == slug)
    }

    pub fn label(self) -> &'static str {
        match self {
            Density::Compact => "Compact",
            Density::Wide => "Wide",
        }
    }

    /// The other layout, which a press of the list's toggle switches to.
    pub fn other(self) -> Self {
        match self {
            Density::Compact => Density::Wide,
            Density::Wide => Density::Compact,
        }
    }

    /// The layout named as rows: `Compact rows`. The list's toggle is named for the layout that
    /// a press switches to; the live line names the layout that is now shown.
    pub fn rows_label(self) -> &'static str {
        match self {
            Density::Compact => "Compact rows",
            Density::Wide => "Wide rows",
        }
    }
}

/// Reads the stored choice. `Wide` when none is stored, which is the layout every existing
/// profile already had.
pub async fn load() -> Density {
    store::spell_density()
        .await
        .ok()
        .flatten()
        .and_then(|slug| Density::from_slug(&slug))
        .unwrap_or_default()
}

/// Saves the choice. A failure is logged and the page keeps the choice for this session.
pub async fn set(density: Density) {
    if let Err(error) = store::set_spell_density(density.slug()).await {
        leptos::logging::warn!("cannot save the spell-row density: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_round_trip_and_unknown_values_fall_back_to_wide() {
        for density in Density::ALL {
            assert_eq!(Density::from_slug(density.slug()), Some(density));
        }
        assert_eq!(Density::from_slug("comfortable"), None);
        assert_eq!(Density::default(), Density::Wide);
    }

    #[test]
    fn the_stored_key_is_the_other_reader_preferences_key() {
        assert_eq!(store::KEY_SPELL_DENSITY, "spell_density");
    }

    #[test]
    fn the_toggle_is_named_for_the_layout_a_press_switches_to() {
        for density in Density::ALL {
            assert_ne!(density.other(), density);
            assert_eq!(density.other().other(), density);
        }
        assert_eq!(Density::Wide.other().rows_label(), "Compact rows");
        assert_eq!(Density::Compact.other().rows_label(), "Wide rows");
    }
}
