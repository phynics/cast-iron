//! Small helpers shared by views.

use cast_iron_schema::EntryKind;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::future::Future;

/// Runs a future once and exposes its result: `None` while loading. Works for results that are
/// not `Send`, such as `Rc` values, because the signal is local to the browser thread.
pub fn local_resource<T: 'static>(
    future: impl Future<Output = T> + 'static,
) -> ReadSignal<Option<T>, LocalStorage> {
    let (read, write) = signal_local(None::<T>);
    spawn_local(async move { write.set(Some(future.await)) });
    read
}

/// Singular display name of a kind, for chips and result lines.
pub fn kind_name(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Rule => "Rule",
        EntryKind::Spell => "Spell",
        EntryKind::Feat => "Feat",
        EntryKind::Creature => "Creature",
        EntryKind::Item => "Item",
        EntryKind::Class => "Class",
        EntryKind::Race => "Race",
        EntryKind::Skill => "Skill",
        EntryKind::Affliction => "Affliction",
        EntryKind::Trap => "Trap",
    }
}

/// Sets `document.title`. Every screen calls this so that history entries read well.
pub fn set_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        document.set_title(&if title.is_empty() {
            "Cast Iron".to_string()
        } else {
            format!("{title} · Cast Iron")
        });
    }
}

/// The first letter of a title for A to Z grouping: an uppercase ASCII letter, else `#`.
pub fn group_letter(title: &str) -> char {
    title
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .filter(char::is_ascii_uppercase)
        .unwrap_or('#')
}

/// Case-insensitive sort key for titles.
pub fn sort_key(title: &str) -> String {
    title.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_group_titles() {
        assert_eq!(group_letter("Fireball"), 'F');
        assert_eq!(group_letter("  acid arrow"), 'A');
        assert_eq!(group_letter("1st Level"), '#');
        assert_eq!(group_letter("(Special)"), 'S');
        assert_eq!(group_letter("Éclair"), '#');
    }
}
