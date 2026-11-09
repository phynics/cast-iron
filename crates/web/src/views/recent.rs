//! Recently viewed entries, kept in the `meta` store under `recent`.
//!
//! Updates go through the `recents` signal that [`crate::app::AppCtx`] holds for the whole page.
//! The entry screen records a visit and Quick search shows the list; both use that one signal, so
//! a visit whose write lands after the layer has read the list still appears there, without a
//! reload.

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
/// Once writes settle, the stored list is the signal's current value. Reading the store back
/// would give two writes the same old list, and the last writer would drop the other visit.
#[derive(Debug, Default, PartialEq, Eq)]
struct Flush {
    in_flight: bool,
    stale: bool,
    /// A write has been requested on this page. The startup read uses it to tell whether the
    /// store may already hold a list that predates its merge.
    wrote: bool,
    /// Bumped by every clear. The startup read captures it before it reads, so history that the
    /// read answers with cannot come back after a clear that landed during that read.
    clear_epoch: u64,
}

impl Flush {
    /// Starts a write when none is in flight. When one is, marks it stale and lets that writer
    /// pick up this call's value; the caller then has nothing more to do.
    fn request(&mut self) -> bool {
        self.wrote = true;
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

    /// The startup read merged the stored history into the signal. A write that is still running
    /// snapshotted the list before that merge, so make it take the merged value. A write that
    /// already finished cannot be corrected that way, so the caller writes the merged list again.
    /// Returns true only in that second case, because a running writer drains it on its own.
    fn merged(&mut self) -> bool {
        if self.in_flight {
            self.stale = true;
            false
        } else {
            self.wrote
        }
    }

    /// The epoch a startup read must capture before it reads.
    fn clear_epoch(&self) -> u64 {
        self.clear_epoch
    }

    /// A clear starts. A startup read that began earlier must not merge the history it answers
    /// with, because that history is older than this clear.
    fn cleared(&mut self) {
        self.clear_epoch = self.clear_epoch.wrapping_add(1);
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

/// Drops every visit that `matches` names, keeping the order of the rest.
pub fn remove_matching(list: &mut Vec<Recent>, matches: impl Fn(&EntryId) -> bool) {
    list.retain(|item| !matches(&item.id));
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

/// Reads the stored history into `recents`. The app calls this once, at startup. A visit that
/// was recorded while the read was in flight makes the running writer take the merged list, and
/// a write that already finished makes this function write the merged list, so the store keeps
/// both the visit and the stored history either way. A clear that lands during the read wins:
/// the history the read answers with is not merged back after it.
pub async fn load_into(recents: RwSignal<Vec<Recent>>) {
    let epoch = FLUSH.with(|flush| flush.borrow().clear_epoch());
    let stored = load().await;
    if FLUSH.with(|flush| flush.borrow().clear_epoch()) != epoch {
        return;
    }
    let before = recents.get_untracked();
    recents.update(|current| merge_stored(current, stored));
    if recents.get_untracked() == before {
        return;
    }
    if FLUSH.with(|flush| flush.borrow_mut().merged()) {
        persist(recents).await;
    }
}

/// Records a visit in `recents` and asks for the list to be written. A visit that lands while
/// another write is in flight marks that writer stale and returns before its own visit is
/// durably written; the running writer persists it on its next pass, so the row still appears
/// without a reload.
pub async fn record(recents: RwSignal<Vec<Recent>>, id: EntryId, title: String) {
    recents.update(|current| push_recent(current, Recent { id, title }));
    persist(recents).await;
}

/// Empties the history and writes the empty list, so the clear survives a reload. A visit that
/// follows the clear is recorded after it and is what the store receives.
pub async fn clear(recents: RwSignal<Vec<Recent>>) {
    FLUSH.with(|flush| flush.borrow_mut().cleared());
    recents.set(Vec::new());
    persist(recents).await;
}

/// Drops every stored visit that `matches` names, so removing a row shown under an alias also
/// forgets the legacy id it was stored under.
pub async fn remove(recents: RwSignal<Vec<Recent>>, matches: impl Fn(&EntryId) -> bool) {
    recents.update(|list| remove_matching(list, matches));
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
    fn removing_a_visit_keeps_the_order_of_the_rest() {
        let mut list = vec![r(3), r(2), r(1)];
        remove_matching(&mut list, |id| id == &r(2).id);
        assert_eq!(list, vec![r(3), r(1)]);
        // An id that is not there changes nothing.
        remove_matching(&mut list, |id| id == &r(9).id);
        assert_eq!(list, vec![r(3), r(1)]);
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

        fn start_write(&mut self) {
            if self.flush.request() {
                self.pending = Some(self.list.clone());
            }
        }

        fn apply(&mut self, act: Act) {
            match act {
                Act::Visit(n) => push_recent(&mut self.list, r(n)),
                Act::Clear => {
                    self.flush.cleared();
                    self.list.clear();
                }
            }
            self.start_write();
        }

        /// The store receives the snapshot the writer took.
        fn commit(&mut self) {
            let snapshot = self.pending.take().expect("a write is in flight");
            self.written.push(snapshot);
        }

        /// The write finishes and the writer takes another pass when the list changed during it.
        fn end(&mut self) {
            if self.flush.finished() {
                self.pending = Some(self.list.clone());
            }
        }

        fn finish(&mut self) {
            self.commit();
            self.end();
        }

        fn clear_epoch(&self) -> u64 {
            self.flush.clear_epoch()
        }

        /// Models [`load_into`]: `epoch` is the clear generation captured before the read, and
        /// `stored` is the answer to that read.
        fn load_into(&mut self, epoch: u64, stored: Vec<Recent>) {
            if self.flush.clear_epoch() != epoch {
                return;
            }
            let before = self.list.clone();
            merge_stored(&mut self.list, stored);
            if self.list == before {
                return;
            }
            if self.flush.merged() {
                self.start_write();
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

    #[test]
    fn a_visit_written_before_the_startup_merge_keeps_the_stored_history() {
        let mut sim = Sim::new();
        let epoch = sim.clear_epoch();
        // The visit's write reaches the store and finishes before the startup read answers.
        sim.apply(Act::Visit(1));
        sim.finish();
        sim.load_into(epoch, vec![r(2), r(3)]);
        // The merge must write itself, because no running writer is left to correct.
        sim.finish();
        assert_eq!(sim.last_written(), [r(1), r(2), r(3)]);
        assert_eq!(sim.list, [r(1), r(2), r(3)]);
    }

    #[test]
    fn a_running_visit_write_takes_the_startup_history_on_its_next_pass() {
        let mut sim = Sim::new();
        let epoch = sim.clear_epoch();
        sim.apply(Act::Visit(1));
        sim.commit();
        sim.load_into(epoch, vec![r(2)]);
        sim.end();
        sim.finish();
        assert_eq!(sim.last_written(), [r(1), r(2)]);
    }

    #[test]
    fn a_clear_during_the_startup_read_is_not_undone_by_the_answer() {
        let mut sim = Sim::new();
        let epoch = sim.clear_epoch();
        sim.apply(Act::Clear);
        sim.load_into(epoch, vec![r(2)]);
        sim.finish();
        assert!(sim.last_written().is_empty());
        assert!(sim.list.is_empty());
    }

    #[test]
    fn a_visit_and_then_a_clear_during_the_startup_read_stay_empty() {
        let mut sim = Sim::new();
        let epoch = sim.clear_epoch();
        sim.apply(Act::Visit(1));
        sim.apply(Act::Clear);
        sim.load_into(epoch, vec![r(2)]);
        sim.finish();
        sim.finish();
        assert!(sim.last_written().is_empty());
        assert!(sim.list.is_empty());
    }

    #[test]
    fn a_visit_after_a_clear_during_the_startup_read_keeps_only_that_visit() {
        let mut sim = Sim::new();
        let epoch = sim.clear_epoch();
        sim.apply(Act::Clear);
        sim.apply(Act::Visit(1));
        sim.load_into(epoch, vec![r(2)]);
        sim.finish();
        sim.finish();
        assert_eq!(sim.last_written(), [r(1)]);
        assert_eq!(sim.list, [r(1)]);
    }
}
