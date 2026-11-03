//! Theme preference: light, dark, or follow the system. The choice is stored in IndexedDB
//! (the source of truth) and mirrored in `localStorage` so that the next launch can apply it
//! synchronously, before the first render.

use crate::store;

const MIRROR_KEY: &str = "cast-iron-theme";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];

    pub fn slug(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.slug() == slug)
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::System => "Match system",
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }
}

/// Sets `data-theme` on `<html>`. The system theme removes the attribute, and the CSS
/// then follows `prefers-color-scheme`.
pub fn apply(theme: Theme) {
    let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    else {
        return;
    };
    match theme {
        Theme::System => {
            let _ = root.remove_attribute("data-theme");
        }
        other => {
            let _ = root.set_attribute("data-theme", other.slug());
        }
    }
}

fn mirror() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// The mirrored preference, readable synchronously. `System` when none is stored.
pub fn mirrored() -> Theme {
    mirror()
        .and_then(|s| s.get_item(MIRROR_KEY).ok().flatten())
        .and_then(|slug| Theme::from_slug(&slug))
        .unwrap_or_default()
}

/// Applies the mirrored theme. Call before the first render.
pub fn apply_mirrored() -> Theme {
    let theme = mirrored();
    apply(theme);
    theme
}

/// Applies a theme, mirrors it, and saves it to IndexedDB.
pub async fn set(theme: Theme) {
    apply(theme);
    if let Some(storage) = mirror() {
        let _ = storage.set_item(MIRROR_KEY, theme.slug());
    }
    if let Err(error) = store::set_theme_preference(theme.slug()).await {
        leptos::logging::warn!("cannot save the theme: {error}");
    }
}

/// Reads the stored preference, which wins over the mirror, and applies it.
pub async fn load() -> Theme {
    let theme = match store::theme_preference().await {
        Ok(Some(slug)) => Theme::from_slug(&slug).unwrap_or_default(),
        _ => mirrored(),
    };
    apply(theme);
    if let Some(storage) = mirror() {
        let _ = storage.set_item(MIRROR_KEY, theme.slug());
    }
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_round_trip() {
        for theme in Theme::ALL {
            assert_eq!(Theme::from_slug(theme.slug()), Some(theme));
        }
        assert_eq!(Theme::from_slug("sepia"), None);
    }
}
