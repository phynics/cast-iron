//! Recently viewed entries, kept in the `meta` store under `recent`.

use crate::store;
pub use crate::store::Recent;
use cast_iron_schema::EntryId;

pub const KEY: &str = store::KEY_RECENT;
pub const MAX_RECENT: usize = 12;

/// Puts `item` first, drops an older copy of it, and keeps at most [`MAX_RECENT`].
pub fn push_recent(list: &mut Vec<Recent>, item: Recent) {
    list.retain(|r| r.id != item.id);
    list.insert(0, item);
    list.truncate(MAX_RECENT);
}

pub async fn load() -> Vec<Recent> {
    store::get_meta::<Vec<Recent>>(KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub async fn record(id: EntryId, title: String) {
    let mut list = load().await;
    push_recent(&mut list, Recent { id, title });
    let _ = store::set_meta(KEY, &list).await;
}

/// Empties the history. The empty list is stored, so the clear survives a reload.
pub async fn clear() {
    let _ = store::set_meta(KEY, &Vec::<Recent>::new()).await;
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
}
