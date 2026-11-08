//! Recently viewed entries, kept in the `meta` store under `recent`.
//!
//! Updates go through the `recents` signal that [`crate::app::AppCtx`] holds for the whole page.
//! The entry screen records a visit and Home shows the list; both use that one signal, so a visit
//! whose write lands after Home has read the store still appears there, without a reload.

use crate::store;
pub use crate::store::Recent;
use cast_iron_schema::EntryId;
use leptos::prelude::*;

pub const KEY: &str = store::KEY_RECENT;
pub const MAX_RECENT: usize = 12;

/// Puts `item` first, drops an older copy of it, and keeps at most [`MAX_RECENT`].
pub fn push_recent(list: &mut Vec<Recent>, item: Recent) {
    list.retain(|r| r.id != item.id);
    list.insert(0, item);
    list.truncate(MAX_RECENT);
}

/// Adds the stored history after the visits already in `current` (which are newer), dropping a
/// copy that is already there, so a read that started before a visit landed cannot hide it.
pub fn merge_stored(current: &mut Vec<Recent>, stored: Vec<Recent>) {
    for item in stored {
        if !current.iter().any(|r| r.id == item.id) {
            current.push(item);
        }
    }
    current.truncate(MAX_RECENT);
}

pub async fn load() -> Vec<Recent> {
    store::get_meta::<Vec<Recent>>(KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Reads the stored history into `recents`. The app calls this once, at startup.
pub async fn load_into(recents: RwSignal<Vec<Recent>>) {
    let stored = load().await;
    recents.update(|current| merge_stored(current, stored));
}

/// Records a visit in the store and in `recents`. The signal update comes after the write, so a
/// screen that mounted while the write was in flight renders the row without a reload.
pub async fn record(recents: RwSignal<Vec<Recent>>, id: EntryId, title: String) {
    let item = Recent { id, title };
    let mut list = load().await;
    push_recent(&mut list, item.clone());
    let _ = store::set_meta(KEY, &list).await;
    recents.update(|current| push_recent(current, item));
}

/// Empties the history. The empty list is stored, so the clear survives a reload.
pub async fn clear(recents: RwSignal<Vec<Recent>>) {
    let _ = store::set_meta(KEY, &Vec::<Recent>::new()).await;
    recents.set(Vec::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: usize) -> Recent {
        Recent {
            id: EntryId::parse(format!("a/e-{n}")).unwrap(),
            title: format!("E{n}"),
        }
    }

    #[test]
    fn newest_first_without_duplicates_and_capped() {
        let mut list = Vec::new();
        for n in 0..20 {
            push_recent(&mut list, r(n));
        }
        assert_eq!(list.len(), MAX_RECENT);
        assert_eq!(list[0], r(19));
        push_recent(&mut list, r(10));
        assert_eq!(list[0], r(10));
        assert_eq!(list.iter().filter(|x| x.id == r(10).id).count(), 1);
        assert_eq!(list.len(), MAX_RECENT);
    }

    #[test]
    fn a_stored_read_does_not_displace_a_newer_visit() {
        // A visit landed while the store was being read, so the read answers without it.
        let mut current = vec![r(3)];
        merge_stored(&mut current, vec![r(2), r(1)]);
        assert_eq!(current, vec![r(3), r(2), r(1)]);
    }

    #[test]
    fn a_stored_read_that_already_has_the_visit_adds_nothing_twice() {
        let mut current = vec![r(3)];
        merge_stored(&mut current, vec![r(3), r(2)]);
        assert_eq!(current, vec![r(3), r(2)]);
    }

    #[test]
    fn the_merged_list_stays_capped() {
        let mut current = vec![r(100)];
        merge_stored(&mut current, (0..MAX_RECENT + 5).rev().map(r).collect());
        assert_eq!(current.len(), MAX_RECENT);
        assert_eq!(current[0], r(100));
    }
}
