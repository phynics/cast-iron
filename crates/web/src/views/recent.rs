//! Recently viewed entries, kept in the `meta` store under `recent`.
//!
//! Updates go through the `recents` signal that [`crate::app::AppCtx`] holds for the whole page.
//! The entry screen records a visit and Home shows the list; both use that one signal, so a visit
//! whose write lands after Home has read the store still appears there, without a reload.

use crate::store;
pub use crate::store::Recent;
use cast_iron_schema::EntryId;
use leptos::prelude::*;
use std::cell::RefCell;

pub const KEY: &str = store::KEY_RECENT;
pub const MAX_RECENT: usize = 12;

/// The write queue. At most one store write runs at a time. A visit or a clear that lands while
/// that write is in flight marks the write stale; the writer then writes the signal's newer
/// value, so a later operation supersedes an earlier one instead of racing it.
///
/// The stored list is always the signal's current value. Reading the store back would give two
/// writes the same old list, and the last writer would drop the other visit.
#[derive(Debug, Default, PartialEq, Eq)]
struct Flush {
    in_flight: bool,
    stale: bool,
}

impl Flush {
    /// Starts a write when none is in flight. When one is, marks it stale and lets that writer
    /// pick up this call's value; the caller then has nothing more to do.
    fn request(&mut self) -> bool {
        if self.in_flight {
            self.stale = true;
            false
        } else {
            self.in_flight = true;
            true
        }
    }

    /// Ends one write. True when the list changed during it, so another write is needed.
    fn finished(&mut self) -> bool {
        if self.stale {
            self.stale = false;
            true
        } else {
            self.in_flight = false;
            false
        }
    }
}

thread_local! {
    static FLUSH: RefCell<Flush> = RefCell::new(Flush::default());
}

/// Writes the signal's current list, once, or again when a mutation landed during the write.
async fn persist(recents: RwSignal<Vec<Recent>>) {
    if !FLUSH.with(|flush| flush.borrow_mut().request()) {
        return;
    }
    loop {
        let list = recents.get_untracked();
        let _ = store::set_meta(KEY, &list).await;
        if !FLUSH.with(|flush| flush.borrow_mut().finished()) {
            return;
        }
    }
}

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

/// Records a visit in `recents`, then writes that list. A visit that lands during the write is
/// merged in the signal first, so the next write includes both, and a screen that mounted while
/// a write was in flight renders the row without a reload.
pub async fn record(recents: RwSignal<Vec<Recent>>, id: EntryId, title: String) {
    recents.update(|current| push_recent(current, Recent { id, title }));
    persist(recents).await;
}

/// Empties the history and writes the empty list, so the clear survives a reload. A visit that
/// follows the clear is recorded after it and is what the store receives.
pub async fn clear(recents: RwSignal<Vec<Recent>>) {
    recents.set(Vec::new());
    persist(recents).await;
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

    /// What a mutation does to the signal list.
    #[derive(Clone, Copy)]
    enum Act {
        Visit(usize),
        Clear,
    }

    /// Models the async split: the signal changes at once, while the store write finishes later.
    /// `finish` completes the one write in flight; the writer then starts another only when a
    /// mutation landed during it, exactly as [`persist`] does.
    struct Sim {
        list: Vec<Recent>,
        flush: Flush,
        pending: Option<Vec<Recent>>,
        written: Vec<Vec<Recent>>,
    }

    impl Sim {
        fn new() -> Self {
            Self {
                list: Vec::new(),
                flush: Flush::default(),
                pending: None,
                written: Vec::new(),
            }
        }

        fn apply(&mut self, act: Act) {
            match act {
                Act::Visit(n) => push_recent(&mut self.list, r(n)),
                Act::Clear => self.list.clear(),
            }
            if self.flush.request() {
                self.pending = Some(self.list.clone());
            }
        }

        fn finish(&mut self) {
            let snapshot = self.pending.take().expect("a write is in flight");
            self.written.push(snapshot);
            if self.flush.finished() {
                self.pending = Some(self.list.clone());
            }
        }

        fn last_written(&self) -> &[Recent] {
            self.written.last().expect("at least one write")
        }
    }

    #[test]
    fn two_quick_visits_are_both_persisted() {
        let mut sim = Sim::new();
        sim.apply(Act::Visit(1));
        // The second visit lands while the first write is in flight.
        sim.apply(Act::Visit(2));
        sim.finish();
        sim.finish();
        assert_eq!(sim.last_written(), [r(2), r(1)]);
        assert_eq!(sim.list, [r(2), r(1)]);
    }

    #[test]
    fn a_clear_then_a_visit_persists_the_visit_and_not_the_cleared_one() {
        let mut sim = Sim::new();
        sim.apply(Act::Visit(1));
        // Both land before the first write completes.
        sim.apply(Act::Clear);
        sim.apply(Act::Visit(2));
        sim.finish();
        sim.finish();
        assert_eq!(sim.last_written(), [r(2)]);
        assert_eq!(sim.list, [r(2)]);
    }

    #[test]
    fn a_visit_then_a_clear_leaves_the_store_empty() {
        let mut sim = Sim::new();
        sim.apply(Act::Visit(1));
        sim.apply(Act::Clear);
        sim.finish();
        sim.finish();
        assert!(sim.last_written().is_empty());
    }
}
