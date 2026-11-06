//! Recent searches: the queries that the reader submitted, kept in the `meta` store under
//! `recent_searches`, next to the recently viewed entries. They stay on the device.

use crate::store;

pub const KEY: &str = store::KEY_RECENT_SEARCHES;
pub const MAX_RECENT_SEARCHES: usize = 10;

/// The text that is stored and compared: trimmed, with each run of whitespace made one space.
pub fn normalize(query: &str) -> String {
    query.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Puts `query` first, drops an older copy of it (case does not matter; the newest spelling
/// stays), and keeps at most [`MAX_RECENT_SEARCHES`]. An empty query changes nothing.
pub fn push_search(list: &mut Vec<String>, query: &str) {
    let query = normalize(query);
    if query.is_empty() {
        return;
    }
    list.retain(|item| !same(item, &query));
    list.insert(0, query);
    list.truncate(MAX_RECENT_SEARCHES);
}

/// Removes one query, however it was capitalized.
pub fn remove_search(list: &mut Vec<String>, query: &str) {
    list.retain(|item| !same(item, query));
}

pub async fn load() -> Vec<String> {
    let mut list = store::get_meta::<Vec<String>>(KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    // A list written by a future version could be longer or hold empty rows.
    list.retain(|item| !item.trim().is_empty());
    list.truncate(MAX_RECENT_SEARCHES);
    list
}

async fn save(list: &[String]) {
    let _ = store::set_meta(KEY, &list.to_vec()).await;
}

/// Adds a submitted query and returns the new list.
pub async fn record(query: &str) -> Vec<String> {
    let mut list = load().await;
    push_search(&mut list, query);
    save(&list).await;
    list
}

/// Removes one query and returns the new list.
pub async fn remove(query: &str) -> Vec<String> {
    let mut list = load().await;
    remove_search(&mut list, query);
    save(&list).await;
    list
}

pub async fn clear() {
    save(&[]).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(list: &[String]) -> Vec<&str> {
        list.iter().map(String::as_str).collect()
    }

    #[test]
    fn newest_first() {
        let mut list = Vec::new();
        for query in ["fireball", "grapple", "dragon"] {
            push_search(&mut list, query);
        }
        assert_eq!(texts(&list), ["dragon", "grapple", "fireball"]);
    }

    #[test]
    fn a_repeated_query_moves_to_the_front_once() {
        let mut list = Vec::new();
        for query in ["fireball", "grapple", "dragon", "fireball"] {
            push_search(&mut list, query);
        }
        assert_eq!(texts(&list), ["fireball", "dragon", "grapple"]);
    }

    #[test]
    fn case_and_spacing_do_not_make_a_second_entry() {
        let mut list = Vec::new();
        push_search(&mut list, "Cure Light");
        push_search(&mut list, "  cure   light ");
        assert_eq!(texts(&list), ["cure light"]);
    }

    #[test]
    fn the_list_is_capped_and_drops_the_oldest() {
        let mut list = Vec::new();
        for n in 0..25 {
            push_search(&mut list, &format!("query {n}"));
        }
        assert_eq!(list.len(), MAX_RECENT_SEARCHES);
        assert_eq!(list[0], "query 24");
        assert_eq!(list[MAX_RECENT_SEARCHES - 1], "query 15");
        // A repeat at the cap does not push anything else out.
        push_search(&mut list, "query 20");
        assert_eq!(list.len(), MAX_RECENT_SEARCHES);
        assert_eq!(list[0], "query 20");
        assert_eq!(list[MAX_RECENT_SEARCHES - 1], "query 15");
    }

    #[test]
    fn an_empty_query_is_ignored() {
        let mut list = vec!["fireball".to_string()];
        push_search(&mut list, "");
        push_search(&mut list, "   ");
        assert_eq!(texts(&list), ["fireball"]);
    }

    #[test]
    fn remove_takes_out_one_query_and_keeps_the_order() {
        let mut list = Vec::new();
        for query in ["a", "b", "c"] {
            push_search(&mut list, query);
        }
        remove_search(&mut list, "B");
        assert_eq!(texts(&list), ["c", "a"]);
        remove_search(&mut list, "missing");
        assert_eq!(texts(&list), ["c", "a"]);
    }
}
